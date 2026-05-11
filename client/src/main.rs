use std::io::{self, Write}; // 标准输入输出及 flush 方法

use anyhow::Result; // 通用错误类型
use prost::Message; // protobuf 消息的编解码 trait
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // 异步读写扩展方法
use tokio::net::TcpStream; // 异步 TCP 流

use bm1_proto::bm1::cs_rpc_msg::Payload; // oneof payload 枚举
use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatReq, PlaceholderReq}; // 命令枚举与消息结构

/// 从流中读取一个完整帧：4 字节大端长度头 + protobuf 消息体
/// 泛型 R: AsyncReadExt + Unpin，支持 TcpStream 和 ReadHalf
async fn read_frame<R: AsyncReadExt + Unpin>(stream: &mut R) -> Result<CsRpcMsg> {
    let len = stream.read_u32().await? as usize; // 读 4 字节长度头
    let mut buf = vec![0u8; len];                // 分配对应大小的缓冲区
    stream.read_exact(&mut buf).await?;          // 读取完整帧体
    Ok(CsRpcMsg::decode(&buf[..])?)              // 反序列化为 CsRpcMsg
}

/// 向流中写入一个完整帧：4 字节大端长度头 + protobuf 消息体
/// 泛型 W: AsyncWriteExt + Unpin，支持 TcpStream 和 WriteHalf
async fn write_frame<W: AsyncWriteExt + Unpin>(stream: &mut W, msg: &CsRpcMsg) -> Result<()> {
    let body = msg.encode_to_vec();                // 序列化消息为字节
    stream.write_u32(body.len() as u32).await?;    // 写长度头
    stream.write_all(&body).await?;                // 写消息体
    Ok(())
}

/// 从标准输入读取一行，带提示符
fn read_line(prompt: &str) -> String {
    print!("{}", prompt);             // 打印提示符
    io::stdout().flush().unwrap();   // 刷新缓冲区，确保提示符立刻显示
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap(); // 读取一行
    input.trim().to_string()         // 去掉首尾空白
}

/// 打印交互菜单
fn print_menu(session_id: u32) {
    println!();
    if session_id > 0 {
        println!("=== session_id: {} ===", session_id); // 会话已建立时显示 ID
    }
    println!("[1] Placeholder  -  发送测试消息");
    println!("[2] Heartbeat    -  发送心跳");
    println!("[3] Reconnect    -  断线重连");
    println!("[0] Exit         -  退出");
    println!();
}

/// 后台读任务：持续从服务端读取帧，处理服务端主动推送的消息（如 HeartbeatReq），
/// 其余响应消息通过 channel 转发给主循环
async fn reader_task(
    mut stream: tokio::io::ReadHalf<TcpStream>, // TCP 读半部分
    tx: tokio::sync::mpsc::Sender<CsRpcMsg>,    // 用于转发响应消息的通道发送端
) {
    loop {
        match read_frame(&mut stream).await { // 读取一帧
            Ok(msg) => match &msg.payload {
                Some(Payload::HeartbeatReq(_)) => { // 服务端主动发来的心跳请求
                    println!("\n<<< 收到服务端心跳 ping, 已自动忽略"); // 提示但不转发
                }
                _ => { // 其他消息（PlaceholderResp、HeartbeatResp 等）
                    if tx.send(msg).await.is_err() { // 通过通道转发给主循环
                        break; // 通道接收端已关闭，退出
                    }
                }
            },
            Err(_) => break, // 读取失败（连接断开），退出
        }
    }
}

/// 连接封装：管理读写通道和后台任务句柄
struct Connection {
    write_tx: tokio::sync::mpsc::Sender<CsRpcMsg>,    // 发送消息的通道（主循环 → 写任务）
    read_rx: tokio::sync::mpsc::Receiver<CsRpcMsg>,   // 接收响应的通道（读任务 → 主循环）
    _reader_handle: tokio::task::JoinHandle<()>,       // 后台读任务句柄（drop 时自动终止）
    _writer_handle: tokio::task::JoinHandle<()>,       // 后台写任务句柄
}

impl Connection {
    /// 通过通道向写任务发送消息
    async fn send(&self, msg: &CsRpcMsg) -> Result<()> {
        self.write_tx.send(msg.clone()).await?; // 发送到通道，写任务会将其写入 TCP
        Ok(())
    }

    /// 从通道接收服务端的响应消息
    async fn recv(&mut self) -> Result<CsRpcMsg> {
        let msg = self
            .read_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed"))?; // 通道关闭则报错
        Ok(msg)
    }
}

/// 建立到服务器的 TCP 连接，启动后台读写任务，返回 Connection 封装
async fn start_connection(addr: &str) -> Result<Connection> {
    let stream = TcpStream::connect(addr).await?; // 连接服务器
    let (read_half, mut write_half) = tokio::io::split(stream); // 拆分为读写两半

    // 创建响应通道：读任务 → 主循环
    let (resp_tx, resp_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);
    // 创建发送通道：主循环 → 写任务
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);

    // 启动后台读任务
    let reader_handle = tokio::spawn(reader_task(read_half, resp_tx));
    // 启动后台写任务
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = write_rx.recv().await { // 从通道取消息
            if write_frame(&mut write_half, &msg).await.is_err() { // 写入 TCP
                break; // 写入失败，退出
            }
        }
    });

    Ok(Connection {
        write_tx,           // 主循环用此发送消息
        read_rx: resp_rx,   // 主循环用此接收响应
        _reader_handle: reader_handle,
        _writer_handle: writer_handle,
    })
}

#[tokio::main] // tokio 异步运行时入口
async fn main() -> Result<()> {
    let mut conn = start_connection("127.0.0.1:8080").await?; // 连接服务器
    println!("connected to server");

    let mut session_id: u32 = 0; // 会话 ID，0 表示尚未建立
    let mut seq: u32 = 0;        // 消息序列号，递增

    loop {
        print_menu(session_id);                 // 打印菜单
        let choice = read_line("选择操作: ");    // 读取用户选择

        match choice.as_str() {
            "1" => {
                let msg = read_line("输入消息内容: "); // 读取消息内容
                seq += 1;                              // 递增序列号
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::Placeholder as i32, // Placeholder 命令
                    seq,                               // 当前序列号
                    session_id,                        // 会话 ID
                    payload: Some(Payload::PlaceholderReq(PlaceholderReq {
                        msg: msg.clone(),              // 用户输入的消息
                    })),
                };
                conn.send(&req).await?;                         // 发送请求
                println!(">>> 发送 PlaceholderReq: {}", msg);

                let resp = conn.recv().await?;                  // 等待响应
                session_id = resp.session_id;                   // 更新会话 ID（首次会从 0 变为服务端分配的值）
                if let Some(Payload::PlaceholderResp(r)) = &resp.payload { // 解析 Placeholder 响应
                    println!("<<< 收到 PlaceholderResp: {}", r.msg);
                }
            }
            "2" => {
                seq += 1; // 递增序列号
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)? // 获取当前 Unix 毫秒时间戳
                    .as_millis() as u64;
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::Heartbeat as i32, // Heartbeat 命令
                    seq,
                    session_id,
                    payload: Some(Payload::HeartbeatReq(HeartbeatReq { timestamp })),
                };
                conn.send(&req).await?;                          // 发送心跳请求
                println!(">>> 发送 HeartbeatReq: timestamp={}", timestamp);

                let resp = conn.recv().await?;                   // 等待心跳响应
                if let Some(Payload::HeartbeatResp(r)) = &resp.payload {
                    println!("<<< 收到 HeartbeatResp: timestamp={}", r.timestamp);
                }
            }
            "3" => {
                println!("--- 断开连接 ---");
                drop(conn); // 释放连接（后台任务自动终止）

                println!("--- 重新连接 ---");
                conn = start_connection("127.0.0.1:8080").await?; // 建立新连接
                println!("connected to server");
            }
            "0" => {
                println!("bye!");
                break; // 退出主循环
            }
            _ => {
                println!("无效选项，请重新选择");
            }
        }
    }

    Ok(())
}
