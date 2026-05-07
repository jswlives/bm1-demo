use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatReq};
use bm1_proto::bm1::cs_rpc_msg::Payload;

use crate::codec;
use crate::handler::{HeartbeatHandler, PlaceholderHandler};
use crate::router::{Context, Router};
use crate::session::SessionManager;

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("server listening on {}", self.addr);

        let session_mgr = Arc::new(Mutex::new(SessionManager::new()));
        let router = Arc::new(Self::build_router());

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("connection from {}", addr);

            let mgr = session_mgr.clone();
            let router = router.clone();

            tokio::spawn(async move {
                handle_connection(stream, mgr, router).await;
            });
        }
    }

    fn build_router() -> Router {
        let mut router = Router::new();
        router.register(CsRpcCmd::Placeholder as i32, Box::new(PlaceholderHandler));
        router.register(CsRpcCmd::Heartbeat as i32, Box::new(HeartbeatHandler));
        router
    }
}

async fn handle_connection(
    stream: TcpStream,
    mgr: Arc<Mutex<SessionManager>>,
    router: Arc<Router>,
) {
    let (reader, writer) = stream.into_split();
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<CsRpcMsg>(32);

    let write_task = tokio::spawn(async move {
        let mut writer = writer;
        while let Some(msg) = write_rx.recv().await {
            if codec::write_frame(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    let mut reader = reader;
    let mut session_id: u32 = 0;
    let mut last_active = tokio::time::Instant::now();
    let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(10));
    let timeout_duration = Duration::from_secs(30);

    loop {
        let deadline = last_active + timeout_duration;

        tokio::select! {
            result = codec::read_frame(&mut reader) => {
                match result {
                    Ok(msg) => {
                        last_active = tokio::time::Instant::now();

                        if session_id == 0 {
                            session_id = if msg.session_id == 0 {
                                mgr.lock().await.create_session()
                            } else if mgr.lock().await.reconnect(msg.session_id) {
                                msg.session_id
                            } else {
                                mgr.lock().await.create_session()
                            };
                            println!("session {} established", session_id);
                        }

                        let ctx = Context { session_id };
                        if let Some(mut resp) = router.dispatch(&ctx, msg) {
                            resp.session_id = session_id;
                            let _ = write_tx.send(resp).await;
                        }
                    }
                    Err(e) => {
                        println!("read error: {}", e);
                        break;
                    }
                }
            }
            _ = heartbeat_interval.tick() => {
                if session_id != 0 {
                    let msg = CsRpcMsg {
                        cmd: CsRpcCmd::Heartbeat as i32,
                        session_id,
                        seq: 0,
                        payload: Some(Payload::HeartbeatReq(HeartbeatReq {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        })),
                    };
                    if write_tx.send(msg).await.is_err() {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                println!("session {} heartbeat timeout", session_id);
                break;
            }
        }
    }

    if session_id != 0 {
        mgr.lock().await.disconnect(session_id);
        println!("session {} disconnected", session_id);
    }
    write_task.abort();
}
