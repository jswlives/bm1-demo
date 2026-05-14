use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use bm1_proto::message::{CsRpcCmd, CsRpcMsg};

use crate::codec;
use crate::handler::LoginHandler;
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

    if session_id != 0 {
        mgr.lock().await.disconnect(session_id);
        println!("session {} disconnected", session_id);
    }
    write_task.abort();
}
