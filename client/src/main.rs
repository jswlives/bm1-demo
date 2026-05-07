use std::time::Duration;

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

#[tokio::main]
async fn main() -> Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("connected to server");

    let mut session_id: u32 = 0;

    // Send placeholder request
    let req = CsRpcMsg {
        cmd: CsRpcCmd::Placeholder as i32,
        seq: 1,
        session_id: 0,
        payload: Some(Payload::PlaceholderReq(PlaceholderReq {
            msg: "hello from client".to_string(),
        })),
    };
    write_frame(&mut stream, &req).await?;

    let resp = read_frame(&mut stream).await?;
    session_id = resp.session_id;
    println!("got response, session_id={}", session_id);

    if let Some(Payload::PlaceholderResp(r)) = &resp.payload {
        println!("placeholder resp: {}", r.msg);
    }

    // Heartbeat loop: send heartbeat every 5 seconds
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    for _ in 0..3 {
        interval.tick().await;

        let hb = CsRpcMsg {
            cmd: CsRpcCmd::Heartbeat as i32,
            seq: 2,
            session_id,
            payload: Some(Payload::HeartbeatReq(HeartbeatReq {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis() as u64,
            })),
        };
        write_frame(&mut stream, &hb).await?;

        let resp = read_frame(&mut stream).await?;
        if let Some(Payload::HeartbeatResp(r)) = &resp.payload {
            println!("heartbeat resp: timestamp={}", r.timestamp);
        }
    }

    // Simulate reconnection
    println!("simulating disconnect...");
    drop(stream);

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("reconnected to server");

    let req = CsRpcMsg {
        cmd: CsRpcCmd::Placeholder as i32,
        seq: 3,
        session_id,
        payload: Some(Payload::PlaceholderReq(PlaceholderReq {
            msg: "hello after reconnect".to_string(),
        })),
    };
    write_frame(&mut stream, &req).await?;

    let resp = read_frame(&mut stream).await?;
    println!("reconnect response, session_id={}", resp.session_id);

    if let Some(Payload::PlaceholderResp(r)) = &resp.payload {
        println!("placeholder resp: {}", r.msg);
    }

    Ok(())
}
