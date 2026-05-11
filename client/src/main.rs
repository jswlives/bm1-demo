use std::io::{self, Write};

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use bm1_proto::bm1::cs_rpc_msg::Payload;
use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatReq, PlaceholderReq};

async fn read_frame(stream: &mut TcpStream) -> Result<CsRpcMsg> {
    let len = stream.read_u32().await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(CsRpcMsg::decode(&buf[..])?)
}

async fn write_frame(stream: &mut TcpStream, msg: &CsRpcMsg) -> Result<()> {
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

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
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
                write_frame(&mut stream, &req).await?;
                println!(">>> 发送 PlaceholderReq: {}", msg);

                let resp = read_frame(&mut stream).await?;
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
                write_frame(&mut stream, &req).await?;
                println!(">>> 发送 HeartbeatReq: timestamp={}", timestamp);

                let resp = read_frame(&mut stream).await?;
                if let Some(Payload::HeartbeatResp(r)) = &resp.payload {
                    println!("<<< 收到 HeartbeatResp: timestamp={}", r.timestamp);
                }
            }
            "3" => {
                println!("--- 断开连接 ---");
                drop(stream);

                println!("--- 重新连接 ---");
                stream = TcpStream::connect("127.0.0.1:8080").await?;
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
