# Server Refactor Design

## Overview

Refactor bm1-server to promote `model` as a top-level module, move business logic to `logic/`, remove heartbeat, add login protocol, and add client-side PlayerData caching.

## 1. Directory Structure (Final)

```
bm1-server/src/
├── main.rs            # Entry point (unchanged)
├── server.rs          # Simplified: remove heartbeat logic
├── session.rs         # Unchanged
├── codec.rs           # Unchanged
├── router.rs          # Unchanged
├── handler/
│   ├── mod.rs         # Export placeholder, login (remove heartbeat)
│   ├── placeholder.rs # Unchanged
│   └── login.rs       # New: parse LoginReq, call logic, return LoginResp
├── logic/
│   ├── mod.rs         # Export login, model
│   └── login.rs       # Login business: query PlayerPool, return result
└── model/
    ├── mod.rs         # Export player, player_pool
    ├── player.rs      # Moved from logic/model/ (unchanged)
    └── player_pool.rs # Moved from logic/model/ (unchanged)
```

## 2. Protocol Changes

### message.proto

- Add `LOGIN_REQ = 3`, `LOGIN_RESP = 4` to `CSRpcCmd`
- Add `LoginReq login_req = 14` and `LoginResp login_resp = 15` to `CSRpcMsg` oneof payload
- Define `LoginReq { uint32 player_id = 1; }`
- Define `LoginResp { PlayerData player_data = 1; string error_msg = 2; }`

### model.proto

No changes needed. `PlayerData` is already defined.

## 3. Remove Heartbeat

- Delete `bm1-server/src/handler/heartbeat.rs`
- Remove heartbeat handler registration from `server.rs` `build_router()`
- Remove heartbeat export from `handler/mod.rs`
- In `server.rs`, simplify the connection loop:
  - Remove 10s heartbeat tick
  - Remove 30s idle timeout
  - Keep only frame reading in `tokio::select!`
- Client: remove heartbeat menu item and send logic

## 4. Login Flow

```
Client sends LoginReq { player_id }
  → Server handler::LoginHandler::handle()
    → Parse payload as LoginReq
    → Call logic::login::handle_login(player_id)
      → Query model::PlayerPool::get(player_id)
      → Found: return (Some(PlayerData), "")
      → Not found: return (None, "player {id} not found")
    → Handler assembles LoginResp CsRpcMsg
  → Client receives LoginResp
    → Success: cache PlayerData
    → Failure: print error_msg
```

### Login logic (logic/login.rs)

```rust
pub fn handle_login(player_id: u32) -> (Option<PlayerData>, String) {
    match PLAYER_POOL.get(player_id) {
        Some(data) => (Some(data), String::new()),
        None => (None, format!("player {} not found", player_id)),
    }
}
```

### PlayerPool initialization

Pre-fill test data so clients can verify login works:
- player_id=1: default PlayerData
- player_id=2: another PlayerData with some items/money

## 5. Client Changes

### Menu (final)

```
1. Placeholder
2. Login
3. Quit
```

### PlayerData cache

```rust
use std::sync::OnceLock;
use std::sync::RwLock;

static PLAYER_DATA: OnceLock<RwLock<Option<PlayerData>>> = OnceLock::new();
```

- On successful login: write PlayerData to cache
- On login failure: do not write, print error_msg
- Other features can read from cache at any time

## 6. Build Steps

1. Update proto files → `cargo build -p bm1-proto`
2. Move model files from `logic/model/` to `model/`
3. Create `logic/login.rs` with login business logic
4. Create `handler/login.rs` with login handler
5. Remove heartbeat handler and simplify server.rs
6. Update client: login menu + PlayerData cache
7. `cargo build` to verify
