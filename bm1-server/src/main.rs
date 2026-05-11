mod codec;   // 编解码模块：TCP 帧的读写（4字节长度头 + protobuf 体）
mod handler; // 处理器模块：各命令的业务逻辑
mod router;  // 路由模块：根据 cmd 分发消息到对应 handler
mod session; // 会话模块：管理客户端会话的创建/重连/断开
mod server;  // 服务器模块：TCP 监听与连接处理

use server::Server; // 引入 Server 结构体

#[tokio::main] // tokio 异步运行时入口
async fn main() {
    // 创建服务器实例，监听所有网卡的 8080 端口
    let server = Server::new("0.0.0.0:8080".to_string());
    // 启动服务器运行循环，打印错误信息（如有）
    if let Err(e) = server.run().await {
        eprintln!("server error: {}", e);
    }
}
