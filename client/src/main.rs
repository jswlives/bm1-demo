use std::io::{self, Write};
use std::sync::RwLock;

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, LoginReq};
use bm1_proto::model::PlayerData;

static PLAYER_DATA: std::sync::OnceLock<RwLock<Option<PlayerData>>> = std::sync::OnceLock::new();

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
    let cache = PLAYER_DATA.get();
    if let Some(guard) = cache.and_then(|c| c.read().ok()) {
        if let Some(data) = guard.as_ref() {
            let base = data.player_base.as_ref();
            println!("  [cached] id={} name={} level={}",
                base.map(|b| b.player_id).unwrap_or(0),
                base.map(|b| b.player_name.as_str()).unwrap_or("-"),
                base.map(|b| b.player_level).unwrap_or(0),
            );
        }
    }
    println!("[1] Login   -  登录");
    println!("[0] Exit    -  退出");
    println!();
}

async fn reader_task(
    mut stream: tokio::io::ReadHalf<TcpStream>,
    tx: tokio::sync::mpsc::Sender<CsRpcMsg>,
) {
    loop {
        match read_frame(&mut stream).await {
            Ok(msg) => {
                if tx.send(msg).await.is_err() {
                    break;
                }
            }
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

    PLAYER_DATA.set(RwLock::new(None)).unwrap();

    let mut session_id: u32 = 0;
    let mut seq: u32 = 0;

    loop {
        print_menu(session_id);
        let choice = read_line("选择操作: ");

        match choice.as_str() {
            "1" => {
                let input = read_line("输入 player_id: ");
                let player_id: u32 = match input.parse() {
                    Ok(id) => id,
                    Err(_) => {
                        println!("无效的 player_id");
                        continue;
                    }
                };

                seq += 1;
                let req = CsRpcMsg {
                    cmd: CsRpcCmd::LoginReq as i32,
                    seq,
                    session_id,
                    payload: Some(Payload::LoginReq(LoginReq { player_id })),
                };
                conn.send(&req).await?;
                println!(">>> 发送 LoginReq: player_id={}", player_id);

                let resp = conn.recv().await?;
                session_id = resp.session_id;

                if let Some(Payload::LoginResp(r)) = &resp.payload {
                    if r.error_msg.is_empty() {
                        if let Some(data) = &r.player_data {
                            let base = data.player_base.as_ref();
                            println!("<<< 登录成功: id={} name={} level={}",
                                base.map(|b| b.player_id).unwrap_or(0),
                                base.map(|b| b.player_name.as_str()).unwrap_or("-"),
                                base.map(|b| b.player_level).unwrap_or(0),
                            );
                            if let Some(cache) = PLAYER_DATA.get() {
                                *cache.write().unwrap() = Some(data.clone());
                            }
                        }
                    } else {
                        println!("<<< 登录失败: {}", r.error_msg);
                    }
                }
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
