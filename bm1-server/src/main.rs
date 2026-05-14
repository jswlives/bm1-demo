mod codec;
mod handler;
mod router;
mod session;
mod server;
mod model;

use server::Server;

#[tokio::main]
async fn main() {
    let server = Server::new("0.0.0.0:8080".to_string());
    if let Err(e) = server.run().await {
        eprintln!("server error: {}", e);
    }
}
