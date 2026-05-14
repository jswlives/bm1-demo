# AddMoney Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an AddMoney command that lets a logged-in player add gold or diamond via a session-bound RPC.

**Architecture:** Client sends `AddMoneyReq { money_type, amount }` over TCP. Server looks up the player via session → player_id mapping, mutates the player's money through `PlayerPool`, and returns `AddMoneyResp { money_count, error_msg }`. Session-player binding is established at login time and stored in `Session`.

**Tech Stack:** Rust, tokio, prost (protobuf), existing bm1-server handler/router pattern.

---

### Task 1: Proto changes + rebuild

**Files:**
- Modify: `share/proto/protos/message.proto`

- [ ] **Step 1: Add enum values, messages, and oneof fields to message.proto**

Append to `CSRpcCmd` enum (after `CS_RPC_CMD_LOGIN_RESP = 4`):

```protobuf
  CS_RPC_CMD_ADD_MONEY_REQ = 5;
  CS_RPC_CMD_ADD_MONEY_RESP = 6;
```

Add new messages (after `LoginResp` message):

```protobuf
message AddMoneyReq {
  model.PlayerBagMoneyType money_type = 1;
  uint32 amount = 2;
}

message AddMoneyResp {
  uint32 money_count = 1;
  string error_msg = 2;
}
```

Add to `payload` oneof (after `LoginResp login_resp = 15`):

```protobuf
    AddMoneyReq add_money_req = 16;
    AddMoneyResp add_money_resp = 17;
```

- [ ] **Step 2: Rebuild proto types**

Run: `cargo build -p bm1-proto`
Expected: BUILD SUCCEED

- [ ] **Step 3: Commit**

```bash
git add share/proto/protos/message.proto
git commit -m "feat(proto): add AddMoney command messages and enum values"
```

---

### Task 2: PlayerPool interior mutability (RwLock)

The global `PlayerPool` is currently behind `LazyLock<PlayerPool>` returning `&PlayerPool`. AddMoneyHandler needs `&mut Player` to call `add_money()`. Wrap in `RwLock` to allow mutation through the global accessor.

**Files:**
- Modify: `bm1-server/src/model/player_pool.rs`
- Modify: `bm1-server/src/handler/login.rs`

- [ ] **Step 1: Write failing test for global mutable access**

Add to `player_pool.rs` tests module:

```rust
#[test]
fn test_global_mut() {
    let mut pool = PlayerPool::global().write().unwrap();
    pool.get_mut(1).unwrap().add_gold(100);
    assert_eq!(pool.get(1).unwrap().gold(), 1100);
    // Restore
    pool.get_mut(1).unwrap().sub_gold(100).unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bm1-server test_global_mut`
Expected: FAIL — `PlayerPool::global()` returns `&PlayerPool`, no `.write()` method

- [ ] **Step 3: Wrap PlayerPool in RwLock**

In `player_pool.rs`, change the import and static:

```rust
use std::sync::{LazyLock, RwLock};
```

Change `PLAYER_POOL` static:

```rust
static PLAYER_POOL: LazyLock<RwLock<PlayerPool>> = LazyLock::new(|| {
    let mut pool = PlayerPool::new();
    pool.load(PlayerData {
        player_base: Some(PlayerBase {
            player_id: 1,
            player_name: "alice".into(),
            player_level: 10,
        }),
        player_bag: Some(PlayerBag {
            items: vec![PlayerBagItem { item_id: 1001, item_count: 5 }],
            money: vec![PlayerBagMoney {
                money_type: PlayerBagMoneyType::Gold as i32,
                money_count: 1000,
            }],
        }),
    });
    pool.load(PlayerData {
        player_base: Some(PlayerBase {
            player_id: 2,
            player_name: "bob".into(),
            player_level: 20,
        }),
        player_bag: Some(PlayerBag {
            items: vec![
                PlayerBagItem { item_id: 2001, item_count: 3 },
                PlayerBagItem { item_id: 2002, item_count: 1 },
            ],
            money: vec![
                PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Gold as i32,
                    money_count: 500,
                },
                PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Diamond as i32,
                    money_count: 50,
                },
            ],
        }),
    });
    RwLock::new(pool)
});
```

Change `global()`:

```rust
pub fn global() -> &'static RwLock<PlayerPool> {
    &PLAYER_POOL
}
```

- [ ] **Step 4: Update LoginHandler to use read lock**

In `login.rs`, change the player lookup:

```rust
let (player_data, error_msg) = {
    let pool = PlayerPool::global().read().unwrap();
    match pool.get(player_id as u64) {
        Some(player) => (Some(player.data().clone()), String::new()),
        None => (None, format!("player {} not found", player_id)),
    }
};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p bm1-server`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add bm1-server/src/model/player_pool.rs bm1-server/src/handler/login.rs
git commit -m "refactor: wrap PlayerPool in RwLock for interior mutability"
```

---

### Task 3: Session player_id field + tests

**Files:**
- Modify: `bm1-server/src/session.rs`

- [ ] **Step 1: Write failing test for player_id on session**

Add to `session.rs` tests module:

```rust
#[test]
fn test_session_player_id() {
    let mut mgr = SessionManager::new();
    let id = mgr.create_session();
    assert_eq!(mgr.player_id(id), None);

    mgr.set_player_id(id, 42);
    assert_eq!(mgr.player_id(id), Some(42));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p bm1-server test_session_player_id`
Expected: FAIL — no `player_id` / `set_player_id` methods on `SessionManager`

- [ ] **Step 3: Add player_id to Session and SessionManager methods**

In `Session` struct, add field:

```rust
pub struct Session {
    pub id: u32,
    pub connected: bool,
    pub last_active: Instant,
    pub player_id: u64,
}
```

In `create_session`, initialize it:

```rust
Session {
    id,
    connected: true,
    last_active: Instant::now(),
    player_id: 0,
},
```

Add methods to `SessionManager`:

```rust
pub fn player_id(&self, id: u32) -> Option<u64> {
    self.sessions.get(&id).map(|s| s.player_id)
}

pub fn set_player_id(&mut self, id: u32, player_id: u64) {
    if let Some(session) = self.sessions.get_mut(&id) {
        session.player_id = player_id;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p bm1-server`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add bm1-server/src/session.rs
git commit -m "feat: add player_id field to Session with getter/setter"
```

---

### Task 4: Context player_id field

**Files:**
- Modify: `bm1-server/src/router.rs`

- [ ] **Step 1: Add player_id to Context**

Change `Context` struct:

```rust
pub struct Context {
    pub session_id: u32,
    pub player_id: u64,
}
```

Update existing `Context` construction in router tests:

```rust
let ctx = Context { session_id: 1, player_id: 0 };
```

(There are two test constructions — update both.)

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p bm1-server`
Expected: ALL PASS (will fail on server.rs too — fix in Task 6)

Note: if `server.rs` fails to compile due to missing `player_id` field, temporarily set it to `0`:

```rust
let ctx = Context { session_id, player_id: 0 };
```

- [ ] **Step 3: Commit**

```bash
git add bm1-server/src/router.rs
git commit -m "feat: add player_id to router Context"
```

---

### Task 5: AddMoneyHandler + tests

**Files:**
- Create: `bm1-server/src/handler/add_money.rs`
- Modify: `bm1-server/src/handler/mod.rs`

- [ ] **Step 1: Write AddMoneyHandler**

Create `bm1-server/src/handler/add_money.rs`:

```rust
use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{AddMoneyResp, CsRpcCmd, CsRpcMsg};
use bm1_proto::model::PlayerBagMoneyType;

use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct AddMoneyHandler;

impl MessageHandler for AddMoneyHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        if ctx.player_id == 0 {
            return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "not logged in".into(),
                })),
            });
        }

        let (money_type, amount) = match &msg.payload {
            Some(Payload::AddMoneyReq(req)) => (req.money_type, req.amount),
            _ => return None,
        };

        let money_type_enum = match PlayerBagMoneyType::try_from(money_type) {
            Ok(t) => t,
            Err(_) => return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "invalid money type".into(),
                })),
            }),
        };

        if money_type_enum == PlayerBagMoneyType::Unspecified {
            return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "invalid money type".into(),
                })),
            });
        }

        let mut pool = PlayerPool::global().write().unwrap();
        let player = match pool.get_mut(ctx.player_id) {
            Some(p) => p,
            None => return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "player not found".into(),
                })),
            }),
        };

        let new_count = player.add_money(money_type_enum, amount);

        Some(CsRpcMsg {
            cmd: CsRpcCmd::AddMoneyResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                money_count: new_count,
                error_msg: String::new(),
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::message::{AddMoneyReq, CsRpcCmd};
    use bm1_proto::model::PlayerBagMoneyType;

    fn make_add_money_msg(money_type: i32, amount: u32) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::AddMoneyReq as i32,
            seq: 1,
            session_id: 1,
            payload: Some(Payload::AddMoneyReq(AddMoneyReq {
                money_type,
                amount,
            })),
        }
    }

    #[test]
    fn test_add_money_not_logged_in() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 0 };
        let msg = make_add_money_msg(PlayerBagMoneyType::Gold as i32, 100);

        let resp = handler.handle(&ctx, msg).unwrap();
        assert_eq!(resp.cmd, CsRpcCmd::AddMoneyResp as i32);
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert_eq!(r.money_count, 0);
            assert!(!r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }
    }

    #[test]
    fn test_add_money_gold() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 1 };
        let msg = make_add_money_msg(PlayerBagMoneyType::Gold as i32, 50);

        let resp = handler.handle(&ctx, msg).unwrap();
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert_eq!(r.money_count, 1050); // alice starts with 1000 gold
            assert!(r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }

        // Restore
        PlayerPool::global().write().unwrap().get_mut(1).unwrap().sub_gold(50).unwrap();
    }

    #[test]
    fn test_add_money_invalid_type() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 1 };
        let msg = make_add_money_msg(0, 100); // Unspecified

        let resp = handler.handle(&ctx, msg).unwrap();
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert!(!r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }
    }
}
```

- [ ] **Step 2: Export AddMoneyHandler in mod.rs**

Change `bm1-server/src/handler/mod.rs` to:

```rust
mod add_money;
mod login;

pub use add_money::AddMoneyHandler;
pub use login::LoginHandler;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p bm1-server`
Expected: ALL PASS (server.rs compile error with missing `player_id` on `Context` construction is acceptable — will be fixed in Task 6. If it blocks compilation, fix `server.rs` Context construction first by adding `player_id: 0`.)

- [ ] **Step 4: Commit**

```bash
git add bm1-server/src/handler/add_money.rs bm1-server/src/handler/mod.rs
git commit -m "feat: add AddMoneyHandler with tests"
```

---

### Task 6: Server integration

Wire up session-player binding in `handle_connection`, populate `ctx.player_id`, and register `AddMoneyHandler` in the router.

**Files:**
- Modify: `bm1-server/src/server.rs`

- [ ] **Step 1: Update server.rs**

Replace the full content of `server.rs` with:

```rust
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg};

use crate::codec;
use crate::handler::{AddMoneyHandler, LoginHandler};
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
                if let Some(mut resp) = router.dispatch(&ctx, msg) {
                    // Bind player_id to session after successful login
                    if login_player_id.is_some() && player_id == 0 {
                        if let Some(Payload::LoginResp(ref login_resp)) = resp.payload {
                            if login_resp.error_msg.is_empty() {
                                mgr.lock().await.set_player_id(session_id, login_player_id.unwrap());
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
```

Key changes from original:
- Import `Payload` and `AddMoneyHandler`
- Register `AddMoneyHandler` in `build_router()`
- Look up `player_id` from session before dispatch
- Extract `login_player_id` from request before dispatch consumes `msg`
- After dispatch, if login succeeded, bind player_id to session
- Pass `player_id` in `Context`

- [ ] **Step 2: Build and run all tests**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
git add bm1-server/src/server.rs
git commit -m "feat: integrate AddMoneyHandler and session-player binding in server"
```

---

### Task 7: Final verification

- [ ] **Step 1: Full build**

Run: `cargo build`
Expected: BUILD SUCCEED

- [ ] **Step 2: All tests**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 3: Quick manual smoke test** (optional)

Run: `cargo run -p bm1-server` in one terminal, `cargo run -p bm1-client` in another. Send Login (player 1), then AddMoney (gold, 100), verify response shows updated balance.
