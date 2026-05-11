# bm1-server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust TCP server with protobuf protocol, session management, reconnection support, cmd routing, and bidirectional heartbeat.

**Architecture:** Tokio async runtime with split TCP streams; prost for protobuf; Handler trait for cmd dispatch with HashMap-based router; SessionManager tracks connection lifecycle; codec layer handles length-delimited framing.

**Tech Stack:** Rust 2024 edition, tokio (full), prost/prost-build 0.13, bytes 1, anyhow 1

---

### Task 1: Set up Cargo Workspace + Proto Crate

**Files:**
- Create: `Cargo.toml` (workspace root, replaces existing)
- Delete: `bm1-server/.git/` (nested repo, must be removed for workspace)
- Modify: `bm1-server/Cargo.toml`
- Create: `share/proto/Cargo.toml`
- Create: `share/proto/build.rs`
- Create: `share/proto/src/lib.rs`
- Create: `share/proto/protos/message.proto`
- Create: `client/Cargo.toml`
- Create: `client/src/main.rs`

- [ ] **Step 1: Remove nested git repo in bm1-server**

```bash
rm -rf bm1-server/.git
```

- [ ] **Step 2: Create workspace root Cargo.toml**

Write to `Cargo.toml`:

```toml
[workspace]
members = [
    "bm1-server",
    "client",
    "share/proto",
]
resolver = "2"
```

- [ ] **Step 3: Update bm1-server/Cargo.toml**

Write to `bm1-server/Cargo.toml`:

```toml
[package]
name = "bm1-server"
version = "0.1.0"
edition = "2024"

[dependencies]
bm1-proto = { path = "../share/proto" }
tokio = { version = "1", features = ["full"] }
prost = "0.13"
bytes = "1"
anyhow = "1"
```

- [ ] **Step 4: Create share/proto/Cargo.toml**

```bash
mkdir -p share/proto/src share/proto/protos
```

Write to `share/proto/Cargo.toml`:

```toml
[package]
name = "bm1-proto"
version = "0.1.0"
edition = "2024"

[dependencies]
prost = "0.13"

[build-dependencies]
prost-build = "0.13"
```

- [ ] **Step 5: Create share/proto/build.rs**

Write to `share/proto/build.rs`:

```rust
fn main() {
    prost_build::Config::new()
        .type_attribute(".", "#[allow(non_camel_case_types)]")
        .compile_protos(&["protos/message.proto"], &["protos/"])
        .unwrap();
}
```

- [ ] **Step 6: Create share/proto/src/lib.rs**

Write to `share/proto/src/lib.rs`:

```rust
pub mod bm1 {
    include!(concat!(env!("OUT_DIR"), "/bm1.rs"));
}
```

- [ ] **Step 7: Create share/proto/protos/message.proto**

Write to `share/proto/protos/message.proto`:

```protobuf
syntax = "proto3";
package bm1;

message CSRpcMsg {
  CSRpcCmd cmd = 1;
  uint32 seq = 2;
  uint32 session_id = 3;
  oneof payload {
    PlaceholderReq placeholder_req = 10;
    PlaceholderResp placeholder_resp = 11;
    HeartbeatReq heartbeat_req = 12;
    HeartbeatResp heartbeat_resp = 13;
  }
}

enum CSRpcCmd {
  CS_RPC_CMD_UNSPECIFIED = 0;
  CS_RPC_CMD_PLACEHOLDER = 1;
  CS_RPC_CMD_HEARTBEAT = 2;
}

message PlaceholderReq {
  string msg = 1;
}

message PlaceholderResp {
  string msg = 1;
}

message HeartbeatReq {
  uint64 timestamp = 1;
}

message HeartbeatResp {
  uint64 timestamp = 1;
}
```

- [ ] **Step 8: Create client/Cargo.toml**

```bash
mkdir -p client/src
```

Write to `client/Cargo.toml`:

```toml
[package]
name = "bm1-client"
version = "0.1.0"
edition = "2024"

[dependencies]
bm1-proto = { path = "../share/proto" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "time", "io-util"] }
prost = "0.13"
anyhow = "1"
```

- [ ] **Step 9: Create client/src/main.rs**

Write to `client/src/main.rs`:

```rust
fn main() {
    println!("client placeholder");
}
```

- [ ] **Step 10: Verify workspace builds**

```bash
cargo build
```

Expected: compiles successfully (bm1-proto generates Rust types from proto, bm1-server and bm1-client build)

- [ ] **Step 11: Inspect generated proto types**

```bash
find target -path "*/bm1.rs" -not -path "*/incremental/*" | head -1 | xargs cat
```

Note: The exact type names (e.g., `CSRpcMsg` vs `CsrpcMsg`) and oneof representation depend on prost output. Use the actual generated names in subsequent tasks.

- [ ] **Step 12: Commit**

```bash
git add Cargo.toml bm1-server/ share/ client/
git commit -m "feat: set up cargo workspace with proto, server, and client crates"
```

---

### Task 2: Implement Codec (Frame Encode/Decode)

**Files:**
- Create: `bm1-server/src/codec.rs`

The codec handles TCP framing: `[4-byte big-endian length][protobuf body]`.

- [ ] **Step 1: Write failing test for codec roundtrip**

Create `bm1-server/src/codec.rs`:

```rust
use anyhow::{Context, Result};
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use bm1_proto::bm1::CsrpcMsg;

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CsrpcMsg> {
    let len = reader
        .read_u32()
        .await
        .context("failed to read frame length")? as usize;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .context("failed to read frame body")?;
    CsrpcMsg::decode(&buf[..]).context("failed to decode CSRpcMsg")
}

pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, msg: &CsrpcMsg) -> Result<()> {
    let body = msg.encode_to_vec();
    writer
        .write_u32(body.len() as u32)
        .await
        .context("failed to write frame length")?;
    writer
        .write_all(&body)
        .await
        .context("failed to write frame body")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::bm1::{CsrpcCmd, PlaceholderReq};

    #[tokio::test]
    async fn test_codec_roundtrip() {
        let (mut client, mut server) = tokio::io::duplex(1024);

        let msg = CsrpcMsg {
            cmd: CsrpcCmd::Placeholder as i32,
            seq: 42,
            session_id: 1,
            payload: Some(bm1_proto::bm1::Payload::PlaceholderReq(PlaceholderReq {
                msg: "hello".to_string(),
            })),
        };

        write_frame(&mut client, &msg).await.unwrap();
        let decoded = read_frame(&mut server).await.unwrap();

        assert_eq!(decoded.cmd, CsrpcCmd::Placeholder as i32);
        assert_eq!(decoded.seq, 42);
        assert_eq!(decoded.session_id, 1);
    }
}
```

Note: Adjust type names (`CsrpcMsg`, `CsrpcCmd`, `Payload`, etc.) based on the actual prost-generated output from Task 1 Step 11. The oneof `payload` may generate a `Payload` enum or individual `Option` fields — check the generated code.

- [ ] **Step 2: Add codec module to main.rs and run test**

Update `bm1-server/src/main.rs`:

```rust
mod codec;

fn main() {
    println!("Hello, world!");
}
```

Run:

```bash
cargo test -p bm1-server
```

Expected: test passes

- [ ] **Step 3: Commit**

```bash
git add bm1-server/src/codec.rs bm1-server/src/main.rs
git commit -m "feat: implement codec with frame encode/decode and roundtrip test"
```

---

### Task 3: Implement SessionManager

**Files:**
- Create: `bm1-server/src/session.rs`

- [ ] **Step 1: Write failing tests for SessionManager**

Create `bm1-server/src/session.rs`:

```rust
use std::collections::HashMap;
use std::time::Instant;

pub struct Session {
    pub id: u32,
    pub connected: bool,
    pub last_active: Instant,
}

pub struct SessionManager {
    sessions: HashMap<u32, Session>,
    next_id: u32,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_session(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.sessions.insert(
            id,
            Session {
                id,
                connected: true,
                last_active: Instant::now(),
            },
        );
        id
    }

    pub fn reconnect(&mut self, id: u32) -> bool {
        if let Some(session) = self.sessions.get_mut(&id) {
            if !session.connected {
                session.connected = true;
                session.last_active = Instant::now();
                return true;
            }
        }
        false
    }

    pub fn disconnect(&mut self, id: u32) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.connected = false;
        }
    }

    pub fn get_session(&self, id: u32) -> Option<&Session> {
        self.sessions.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session();
        assert_eq!(id, 1);
        let session = mgr.get_session(id).unwrap();
        assert!(session.connected);
    }

    #[test]
    fn test_disconnect_and_reconnect() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session();

        mgr.disconnect(id);
        assert!(!mgr.get_session(id).unwrap().connected);

        let result = mgr.reconnect(id);
        assert!(result);
        assert!(mgr.get_session(id).unwrap().connected);
    }

    #[test]
    fn test_reconnect_nonexistent_session() {
        let mut mgr = SessionManager::new();
        assert!(!mgr.reconnect(999));
    }

    #[test]
    fn test_reconnect_already_connected() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session();
        assert!(!mgr.reconnect(id));
    }
}
```

- [ ] **Step 2: Add session module to main.rs and run tests**

Update `bm1-server/src/main.rs`:

```rust
mod codec;
mod session;

fn main() {
    println!("Hello, world!");
}
```

Run:

```bash
cargo test -p bm1-server
```

Expected: all 4 tests pass

- [ ] **Step 3: Commit**

```bash
git add bm1-server/src/session.rs bm1-server/src/main.rs
git commit -m "feat: implement SessionManager with create, disconnect, reconnect"
```

---

### Task 4: Implement Router + Handler Trait

**Files:**
- Create: `bm1-server/src/router.rs`

- [ ] **Step 1: Write Router with Handler trait and tests**

Create `bm1-server/src/router.rs`:

```rust
use std::collections::HashMap;

use bm1_proto::bm1::CsrpcMsg;

pub struct Context {
    pub session_id: u32,
}

pub trait MessageHandler: Send + Sync {
    fn handle(&self, ctx: &Context, msg: CsrpcMsg) -> Option<CsrpcMsg>;
}

pub struct Router {
    handlers: HashMap<i32, Box<dyn MessageHandler>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, cmd: i32, handler: Box<dyn MessageHandler>) {
        self.handlers.insert(cmd, handler);
    }

    pub fn dispatch(&self, ctx: &Context, msg: CsrpcMsg) -> Option<CsrpcMsg> {
        self.handlers.get(&msg.cmd).and_then(|h| h.handle(ctx, msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::bm1::CsrpcCmd;

    struct EchoHandler;

    impl MessageHandler for EchoHandler {
        fn handle(&self, ctx: &Context, msg: CsrpcMsg) -> Option<CsrpcMsg> {
            Some(CsrpcMsg {
                cmd: msg.cmd,
                seq: msg.seq,
                session_id: ctx.session_id,
                ..Default::default()
            })
        }
    }

    #[test]
    fn test_dispatch_known_cmd() {
        let mut router = Router::new();
        router.register(CsrpcCmd::Placeholder as i32, Box::new(EchoHandler));

        let ctx = Context { session_id: 1 };
        let msg = CsrpcMsg {
            cmd: CsrpcCmd::Placeholder as i32,
            seq: 1,
            ..Default::default()
        };

        let resp = router.dispatch(&ctx, msg);
        assert!(resp.is_some());
        assert_eq!(resp.unwrap().session_id, 1);
    }

    #[test]
    fn test_dispatch_unknown_cmd() {
        let router = Router::new();
        let ctx = Context { session_id: 1 };
        let msg = CsrpcMsg {
            cmd: 999,
            ..Default::default()
        };

        let resp = router.dispatch(&ctx, msg);
        assert!(resp.is_none());
    }
}
```

- [ ] **Step 2: Add router module to main.rs and run tests**

Update `bm1-server/src/main.rs`:

```rust
mod codec;
mod session;
mod router;

fn main() {
    println!("Hello, world!");
}
```

Run:

```bash
cargo test -p bm1-server
```

Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add bm1-server/src/router.rs bm1-server/src/main.rs
git commit -m "feat: implement Router with Handler trait and dispatch"
```

---

### Task 5: Implement Handlers

**Files:**
- Create: `bm1-server/src/handler/mod.rs`
- Create: `bm1-server/src/handler/placeholder.rs`
- Create: `bm1-server/src/handler/heartbeat.rs`

- [ ] **Step 1: Create placeholder handler**

Create `bm1-server/src/handler/placeholder.rs`:

```rust
use bm1_proto::bm1::{CsrpcCmd, CsrpcMsg, PlaceholderResp};

use crate::router::{Context, MessageHandler};

pub struct PlaceholderHandler;

impl MessageHandler for PlaceholderHandler {
    fn handle(&self, ctx: &Context, msg: CsrpcMsg) -> Option<CsrpcMsg> {
        let req_msg = match &msg.payload {
            Some(p) => match p {
                bm1_proto::bm1::Payload::PlaceholderReq(req) => &req.msg,
                _ => return None,
            },
            None => return None,
        };

        Some(CsrpcMsg {
            cmd: CsrpcCmd::Placeholder as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(bm1_proto::bm1::Payload::PlaceholderResp(PlaceholderResp {
                msg: format!("echo: {}", req_msg),
            })),
        })
    }
}
```

Note: Adjust `Payload` enum variant names based on actual prost output. If prost generates individual `Option` fields instead of a `Payload` enum, adjust accordingly (e.g., `msg.placeholder_req.as_ref()` instead of pattern matching on `Payload`).

- [ ] **Step 2: Create heartbeat handler**

Create `bm1-server/src/handler/heartbeat.rs`:

```rust
use bm1_proto::bm1::{CsrpcCmd, CsrpcMsg, HeartbeatResp};

use crate::router::{Context, MessageHandler};

pub struct HeartbeatHandler;

impl MessageHandler for HeartbeatHandler {
    fn handle(&self, ctx: &Context, msg: CsrpcMsg) -> Option<CsrpcMsg> {
        let timestamp = match &msg.payload {
            Some(p) => match p {
                bm1_proto::bm1::Payload::HeartbeatReq(req) => req.timestamp,
                _ => 0,
            },
            None => 0,
        };

        Some(CsrpcMsg {
            cmd: CsrpcCmd::Heartbeat as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(bm1_proto::bm1::Payload::HeartbeatResp(HeartbeatResp {
                timestamp,
            })),
        })
    }
}
```

- [ ] **Step 3: Create handler module**

Create `bm1-server/src/handler/mod.rs`:

```rust
mod placeholder;
mod heartbeat;

pub use placeholder::PlaceholderHandler;
pub use heartbeat::HeartbeatHandler;
```

- [ ] **Step 4: Add handler module to main.rs and build**

Update `bm1-server/src/main.rs`:

```rust
mod codec;
mod session;
mod router;
mod handler;

fn main() {
    println!("Hello, world!");
}
```

Run:

```bash
cargo build -p bm1-server
```

Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add bm1-server/src/handler/ bm1-server/src/main.rs
git commit -m "feat: implement placeholder and heartbeat handlers"
```

---

### Task 6: Implement Server + Wire Everything

**Files:**
- Create: `bm1-server/src/server.rs`
- Modify: `bm1-server/src/main.rs`

- [ ] **Step 1: Create server.rs with TCP listener and connection handler**

Create `bm1-server/src/server.rs`:

```rust
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use bm1_proto::bm1::{CsrpcCmd, CsrpcMsg, HeartbeatReq};

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
        router.register(CsrpcCmd::Placeholder as i32, Box::new(PlaceholderHandler));
        router.register(CsrpcCmd::Heartbeat as i32, Box::new(HeartbeatHandler));
        router
    }
}

async fn handle_connection(
    stream: TcpStream,
    mgr: Arc<Mutex<SessionManager>>,
    router: Arc<Router>,
) {
    let (reader, writer) = stream.into_split();
    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<CsrpcMsg>(32);

    // Write task: reads from channel and writes to stream
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

                        // Session creation / reconnection on first message
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

                        // Dispatch to handler
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
                    let msg = CsrpcMsg {
                        cmd: CsrpcCmd::Heartbeat as i32,
                        session_id,
                        payload: Some(bm1_proto::bm1::Payload::HeartbeatReq(HeartbeatReq {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        })),
                        ..Default::default()
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

    // Cleanup
    if session_id != 0 {
        mgr.lock().await.disconnect(session_id);
        println!("session {} disconnected", session_id);
    }
    write_task.abort();
}
```

Note: Adjust `bm1_proto::bm1::Payload` references based on actual prost output. If oneof generates individual `Option` fields, replace `payload: Some(Payload::HeartbeatReq(...))` with `heartbeat_req: Some(HeartbeatReq { ... })`.

- [ ] **Step 2: Update main.rs**

Write to `bm1-server/src/main.rs`:

```rust
mod codec;
mod handler;
mod router;
mod session;
mod server;

use server::Server;

#[tokio::main]
async fn main() {
    let server = Server::new("0.0.0.0:8080".to_string());
    if let Err(e) = server.run().await {
        eprintln!("server error: {}", e);
    }
}
```

- [ ] **Step 3: Build and verify**

```bash
cargo build -p bm1-server
```

Expected: compiles successfully

- [ ] **Step 4: Run server manually to verify it starts**

```bash
cargo run -p bm1-server &
sleep 1
# Verify it prints "server listening on 0.0.0.0:8080"
kill %1
```

Expected: server starts and listens on port 8080

- [ ] **Step 5: Commit**

```bash
git add bm1-server/src/server.rs bm1-server/src/main.rs
git commit -m "feat: implement TCP server with session, router, and heartbeat"
```

---

### Task 7: Implement Client

**Files:**
- Modify: `client/src/main.rs`

- [ ] **Step 1: Implement demo client**

Write to `client/src/main.rs`:

```rust
use std::time::Duration;

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use bm1_proto::bm1::{CsrpcCmd, CsrpcMsg, HeartbeatReq, PlaceholderReq};

async fn read_frame(stream: &mut TcpStream) -> Result<CsrpcMsg> {
    let len = stream.read_u32().await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(CsrpcMsg::decode(&buf[..])?)
}

async fn write_frame(stream: &mut TcpStream, msg: &CsrpcMsg) -> Result<()> {
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
    let req = CsrpcMsg {
        cmd: CsrpcCmd::Placeholder as i32,
        seq: 1,
        session_id: 0,
        payload: Some(bm1_proto::bm1::Payload::PlaceholderReq(PlaceholderReq {
            msg: "hello from client".to_string(),
        })),
    };
    write_frame(&mut stream, &req).await?;

    let resp = read_frame(&mut stream).await?;
    session_id = resp.session_id;
    println!("got response, session_id={}", session_id);

    if let Some(bm1_proto::bm1::Payload::PlaceholderResp(r)) = &resp.payload {
        println!("placeholder resp: {}", r.msg);
    }

    // Heartbeat loop: send heartbeat every 5 seconds
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    for _ in 0..3 {
        interval.tick().await;

        let hb = CsrpcMsg {
            cmd: CsrpcCmd::Heartbeat as i32,
            seq: 2,
            session_id,
            payload: Some(bm1_proto::bm1::Payload::HeartbeatReq(HeartbeatReq {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis() as u64,
            })),
        };
        write_frame(&mut stream, &hb).await?;

        let resp = read_frame(&mut stream).await?;
        if let Some(bm1_proto::bm1::Payload::HeartbeatResp(r)) = &resp.payload {
            println!("heartbeat resp: timestamp={}", r.timestamp);
        }
    }

    // Simulate reconnection: drop connection and reconnect with session_id
    println!("simulating disconnect...");
    drop(stream);

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("reconnected to server");

    let req = CsrpcMsg {
        cmd: CsrpcCmd::Placeholder as i32,
        seq: 3,
        session_id,
        payload: Some(bm1_proto::bm1::Payload::PlaceholderReq(PlaceholderReq {
            msg: "hello after reconnect".to_string(),
        })),
    };
    write_frame(&mut stream, &req).await?;

    let resp = read_frame(&mut stream).await?;
    println!("reconnect response, session_id={}", resp.session_id);

    if let Some(bm1_proto::bm1::Payload::PlaceholderResp(r)) = &resp.payload {
        println!("placeholder resp: {}", r.msg);
    }

    Ok(())
}
```

Note: Adjust `bm1_proto::bm1::Payload` references based on actual prost output, same as Task 5 and 6.

- [ ] **Step 2: Build client**

```bash
cargo build -p bm1-client
```

Expected: compiles successfully

- [ ] **Step 3: Integration test — run server and client together**

Terminal 1:

```bash
cargo run -p bm1-server
```

Terminal 2:

```bash
cargo run -p bm1-client
```

Expected output in client:
- "connected to server"
- "got response, session_id=N"
- "placeholder resp: echo: hello from client"
- heartbeat responses
- "simulating disconnect..."
- "reconnected to server"
- "reconnect response, session_id=N"
- "placeholder resp: echo: hello after reconnect"

Expected output in server:
- "connection from 127.0.0.1:XXXXX"
- "session N established"
- "session N disconnected" (after client drops connection)
- "connection from 127.0.0.1:YYYYY" (reconnect)
- "session N established" (same session ID)

- [ ] **Step 4: Commit**

```bash
git add client/src/main.rs
git commit -m "feat: implement demo client with heartbeat and reconnection"
```
