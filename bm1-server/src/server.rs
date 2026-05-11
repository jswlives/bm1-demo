use std::sync::Arc; // 原子引用计数，用于在多个 tokio 任务间共享所有权
use std::time::Duration; // 时间间隔，用于心跳超时判断

use tokio::net::{TcpListener, TcpStream}; // TCP 监听器与连接流
use tokio::sync::Mutex; // 异步互斥锁，用于保护 SessionManager 的并发访问

use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatReq}; // protobuf 生成的消息类型
use bm1_proto::bm1::cs_rpc_msg::Payload; // oneof payload 枚举

use crate::codec; // 编解码：帧读写
use crate::handler::{HeartbeatHandler, PlaceholderHandler}; // 两个业务处理器
use crate::router::{Context, Router}; // 路由上下文与路由器
use crate::session::SessionManager; // 会话管理器

pub struct Server {
    addr: String, // 监听地址，如 "0.0.0.0:8080"
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self { addr } // 保存监听地址
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        // 绑定 TCP 监听器到指定地址
        let listener = TcpListener::bind(&self.addr).await?;
        println!("server listening on {}", self.addr);

        // 会话管理器，用 Arc<Mutex<>> 包裹以在多个连接任务间共享
        let session_mgr = Arc::new(Mutex::new(SessionManager::new()));
        // 路由器，用 Arc 包裹以在多个连接任务间共享（只读，无需 Mutex）
        let router = Arc::new(Self::build_router());

        loop {
            // 接受新的 TCP 连入连接
            let (stream, addr) = listener.accept().await?;
            println!("connection from {}", addr);

            // 为每个连接克隆共享资源的引用
            let mgr = session_mgr.clone();
            let router = router.clone();

            // 为每个连接独立 spawn 一个异步任务，实现并发处理
            tokio::spawn(async move {
                handle_connection(stream, mgr, router).await;
            });
        }
    }

    // 构建路由表，注册各命令号对应的处理器
    fn build_router() -> Router {
        let mut router = Router::new();
        router.register(CsRpcCmd::Placeholder as i32, Box::new(PlaceholderHandler)); // Placeholder 命令 → PlaceholderHandler
        router.register(CsRpcCmd::Heartbeat as i32, Box::new(HeartbeatHandler));     // Heartbeat 命令 → HeartbeatHandler
        router
    }
}

/// 处理单个客户端连接的完整生命周期
async fn handle_connection(
    stream: TcpStream,                                   // 客户端的 TCP 流
    mgr: Arc<Mutex<SessionManager>>,                     // 共享的会话管理器
    router: Arc<Router>,                                 // 共享的路由器
) {
    // 将 TCP 流拆分为读、写两半，实现独立的双向 I/O
    let (reader, writer) = stream.into_split();
    // 创建消息通道：所有需要写入的数据都通过此通道发送，由写任务统一写 TCP
    // 避免多个协程同时写 TCP 导致数据交错
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);

    // 后台写任务：从通道接收消息并写入 TCP
    let write_task = tokio::spawn(async move {
        let mut writer = writer;
        while let Some(msg) = write_rx.recv().await { // 从通道取消息
            if codec::write_frame(&mut writer, &msg).await.is_err() { // 编码并写入 TCP
                break; // 写入失败（连接断开），退出循环
            }
        }
    });

    let mut reader = reader;     // 读半部分
    let mut session_id: u32 = 0; // 当前连接的会话 ID，0 表示尚未建立会话
    let mut last_active = tokio::time::Instant::now(); // 最后活跃时间，用于超时检测
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(10)); // 每 10 秒触发一次心跳发送
    let timeout_duration = Duration::from_secs(30); // 30 秒无数据则判定超时断开

    loop {
        // 计算当前连接的超时截止时间
        let deadline = last_active + timeout_duration;

        // tokio::select! 同时等待三个事件，谁先就绪就处理谁
        tokio::select! {
            // 事件1：从 TCP 读取一个完整帧
            result = codec::read_frame(&mut reader) => {
                match result {
                    Ok(msg) => {
                        last_active = tokio::time::Instant::now(); // 收到消息，刷新活跃时间

                        // 首条消息时建立会话：session_id==0 表示新会话，非0尝试重连
                        if session_id == 0 {
                            session_id = if msg.session_id == 0 {
                                mgr.lock().await.create_session() // 客户端无 session_id，创建新会话
                            } else if mgr.lock().await.reconnect(msg.session_id) {
                                msg.session_id // 客户端携带已有 session_id，重连成功
                            } else {
                                mgr.lock().await.create_session() // 重连失败（会话不存在或已在线），创建新会话
                            };
                            println!("session {} established", session_id);
                        }

                        // 构建路由上下文（包含当前会话 ID），分发给对应 handler
                        let ctx = Context { session_id };
                        if let Some(mut resp) = router.dispatch(&ctx, msg) {
                            resp.session_id = session_id; // 在响应中填入会话 ID
                            let _ = write_tx.send(resp).await; // 将响应通过通道发给写任务
                        }
                    }
                    Err(e) => {
                        println!("read error: {}", e); // 读取错误（连接断开等）
                        break;
                    }
                }
            }
            // 事件2：心跳定时器触发，每 10 秒向客户端发送一次心跳请求
            _ = heartbeat_interval.tick() => {
                if session_id != 0 { // 会话已建立才发心跳
                    let msg = CsRpcMsg {
                        cmd: CsRpcCmd::Heartbeat as i32, // 心跳命令号
                        session_id,                      // 当前会话 ID
                        seq: 0,                          // 心跳不使用序列号
                        payload: Some(Payload::HeartbeatReq(HeartbeatReq {
                            timestamp: std::time::SystemTime::now() // 当前时间戳
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        })),
                    };
                    if write_tx.send(msg).await.is_err() { // 通过通道发送心跳
                        break; // 通道关闭，写任务已退出
                    }
                }
            }
            // 事件3：超时定时器，30 秒内无任何消息则断开连接
            _ = tokio::time::sleep_until(deadline) => {
                println!("session {} heartbeat timeout", session_id);
                break;
            }
        }
    }

    // 连接结束后，清理会话状态
    if session_id != 0 {
        mgr.lock().await.disconnect(session_id); // 标记会话为断开（允许后续重连）
        println!("session {} disconnected", session_id);
    }
    write_task.abort(); // 终止写任务
}
