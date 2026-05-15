use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::PlayerDataNotify;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg};

use crate::codec;
use crate::handler::{AddMoneyHandler, LoginHandler, SkillUnlockHandler, SkillUpgradeHandler};
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
        router.register(CsRpcCmd::LoginReq as i32, Box::new(LoginHandler));
        router.register(CsRpcCmd::AddMoneyReq as i32, Box::new(AddMoneyHandler));
        router.register(CsRpcCmd::SkillUnlockReq as i32, Box::new(SkillUnlockHandler));
        router.register(CsRpcCmd::SkillUpgradeReq as i32, Box::new(SkillUpgradeHandler));
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

    loop {
        match codec::read_frame(&mut reader).await {
            Ok(msg) => {
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

                let player_id = mgr.lock().await.player_id(session_id).unwrap_or(0);

                // Extract login player_id before dispatch consumes msg
                let login_player_id = match &msg.payload {
                    Some(Payload::LoginReq(req)) => Some(req.player_id as u64),
                    _ => None,
                };

                let ctx = Context { session_id, player_id };

                // Snapshot player data before handler (if player is logged in)
                let before = if player_id > 0 {
                    let pool = crate::model::player_pool::PlayerPool::global().read().unwrap();
                    pool.get(player_id).map(|p| p.data().clone())
                } else {
                    None
                };

                if let Some(mut resp) = router.dispatch(&ctx, msg) {
                    // Bind player_id to session after successful login
                    if login_player_id.is_some() && player_id == 0 {
                        if let Some(Payload::LoginResp(ref login_resp)) = resp.payload {
                            if login_resp.error_msg.is_empty() {
                                mgr.lock().await.set_player_id(session_id, login_player_id.unwrap());
                            }
                        }
                    }

                    // Snapshot diff: send PlayerDataNotify BEFORE Resp
                    if let Some(before_data) = before {
                        let after_data = {
                            let pool = crate::model::player_pool::PlayerPool::global().read().unwrap();
                            pool.get(player_id).map(|p| p.data().clone())
                        };
                        if let Some(after_data) = after_data {
                            if let Some(delta) = crate::model::delta::diff_player_data(&before_data, &after_data) {
                                let notify = CsRpcMsg {
                                    cmd: CsRpcCmd::PlayerDataNotify as i32,
                                    seq: 0,
                                    session_id,
                                    payload: Some(Payload::PlayerDataNotify(
                                        PlayerDataNotify {
                                            delta: Some(delta),
                                            reason: String::new(),
                                        },
                                    )),
                                };
                                let _ = write_tx.send(notify).await;
                            }
                        }
                    }

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

    if session_id != 0 {
        mgr.lock().await.disconnect(session_id);
        println!("session {} disconnected", session_id);
    }
    write_task.abort();
}
