use std::io::{self, Write};

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use bm1_proto::bm1::cs_rpc_msg::Payload;
use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatReq, PlaceholderReq};

async fn read_frame<R: AsyncReadExt + Unpin>(stream: &mut R) -> Result<CsRpcMsg> {
    let len = stream.read_u32().await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(CsRpcMsg::decode(&buf[..])?)
}

async fn write_frame<W: AsyncWriteExt + Unpin>(stream: &mut W, msg: &CsRpcMsg) -> Result<()> {
    let body = msg.encode_to_vec();
    stream.write_u32(body.len() as u32).await?;
    stream.write_all(&body).await?;
    Ok(())
}

fn read_line(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn print_menu(session_id: u32) {
    println!();
    if session_id > 0 {
        println!("=== session_id: {} ===", session_id);
    }
    println!("[1] Placeholder  -  发送测试消息");
    println!("[2] Heartbeat    -  发送心跳");
    println!("[3] Reconnect    -  断线重连");
    println!("[0] Exit         -  退出");
    println!();
}

/// Background reader: reads all frames from server, handles server-pushed
/// messages (e.g. HeartbeatReq) inline, and forwards the rest via `tx`.
async fn reader_task(
    mut stream: tokio::io::ReadHalf<TcpStream>,
    tx: tokio::sync::mpsc::Sender<CsRpcMsg>,
) {
    loop {
        match read_frame(&mut stream).await {
            Ok(msg) => match &msg.payload {
                Some(Payload::HeartbeatReq(_)) => {
                    println!("\n<<< 收到服务端心跳 ping, 已自动忽略");
                }
                _ => {
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
            },
            Err(_) => break,
        }
    }
}

struct Connection {
    write_tx: tokio::sync::mpsc::Sender<CsRpcMsg>,
    read_rx: tokio::sync::mpsc::Receiver<CsRpcMsg>,
    _reader_handle: tokio::task::JoinHandle<()>,
    _writer_handle: tokio::task::JoinHandle<()>,
}

impl Connection {
    async fn send(&self, msg: &CsRpcMsg) -> Result<()> {
        self.write_tx.send(msg.clone()).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CsRpcMsg> {
        let msg = self
            .read_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed"))?;
        Ok(msg)
    }
}

async fn start_connection(addr: &str) -> Result<Connection> {
    let stream = TcpStream::connect(addr).await?;
    let (read_half, mut write_half) = tokio::io::split(stream);

    let (resp_tx, resp_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);

    let reader_handle = tokio::spawn(reader_task(read_half, resp_tx));
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = write_rx.recv().await {
            if write_frame(&mut write_half, &msg).await.is_err() {
                break;
            }
        }
    });

    Ok(Connection {
        write_tx,
        read_rx: resp_rx,
        _reader_handle: reader_handle,
        _writer_handle: writer_handle,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut conn = start_connection("127.0.0.1:8080").await?;
    println!("connected to server");

    let mut session_id: u32 = 0;
    let mut seq: u32 = 0;

    loop {
        print_menu(session_id);
        let choice = read_line("选择操作: ");

        match choice.as_str() {
            "1" => {
                let msg = read_line("输入消息内容: ");
                seq += 1;
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::Placeholder as i32,
                    seq,
                    session_id,
                    payload: Some(Payload::PlaceholderReq(PlaceholderReq {
                        msg: msg.clone(),
                    })),
                };
                conn.send(&req).await?;
                println!(">>> 发送 PlaceholderReq: {}", msg);

                let resp = conn.recv().await?;
                session_id = resp.session_id;
                if let Some(Payload::PlaceholderResp(r)) = &resp.payload {
                    println!("<<< 收到 PlaceholderResp: {}", r.msg);
                }
            }
            "2" => {
                seq += 1;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis() as u64;
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::Heartbeat as i32,
                    seq,
                    session_id,
                    payload: Some(Payload::HeartbeatReq(HeartbeatReq { timestamp })),
                };
                conn.send(&req).await?;
                println!(">>> 发送 HeartbeatReq: timestamp={}", timestamp);

                let resp = conn.recv().await?;
                if let Some(Payload::HeartbeatResp(r)) = &resp.payload {
                    println!("<<< 收到 HeartbeatResp: timestamp={}", r.timestamp);
                }
            }
            "3" => {
                println!("--- 断开连接 ---");
                drop(conn);

                println!("--- 重新连接 ---");
                conn = start_connection("127.0.0.1:8080").await?;
                println!("connected to server");
            }
            "0" => {
                println!("bye!");
                break;
            }
            _ => {
                println!("无效选项，请重新选择");
            }
        }
    }

    Ok(())
}
