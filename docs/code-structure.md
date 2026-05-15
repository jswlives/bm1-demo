# Code Structure

This document records the full project file tree and core API signatures. Update this file when files are added/removed or public APIs change.

## File Tree

```
bm1-demo/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── CLAUDE.md
├── README.md
├── .gitignore
├── .claude/settings.local.json
├── docs/                         # Design docs & plans
│   ├── code-structure.md         # ← This file
│   ├── context-optimization-guide.md
│   └── superpowers/
│       ├── plans/
│       │   ├── 2026-05-07-bm1-server-plan.md
│       │   └── 2026-05-14-add-money-plan.md
│       └── specs/
│           ├── 2026-05-07-bm1-server-design.md
│           ├── 2026-05-14-add-money-design.md
│           └── 2026-05-14-server-refactor-design.md
├── bm1-server/                   # TCP server (package: bm1-server)
│   ├── Cargo.toml
│   ├── .gitignore
│   └── src/
│       ├── main.rs               # Entry point: parses addr, runs Server
│       ├── server.rs             # Server struct: accept loop, build_router(), handle_connection()
│       ├── codec.rs              # read_frame/write_frame: 4-byte len + protobuf
│       ├── router.rs             # Router + Context + MessageHandler trait
│       ├── session.rs            # Session + SessionManager (create/reconnect/disconnect)
│       ├── handler/
│       │   ├── mod.rs            # Re-exports: LoginHandler, AddMoneyHandler
│       │   ├── login.rs          # LoginHandler: lookup player by player_id
│       │   └── add_money.rs      # AddMoneyHandler: add gold/diamond with auth check
│       └── model/
│           ├── mod.rs            # Re-exports: player, player_pool
│           ├── player.rs         # Player: wraps PlayerData, money/item CRUD
│           └── player_pool.rs    # PlayerPool: global LazyLock<RwLock<PlayerPool>>
├── client/                       # Interactive client (package: bm1-client)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs               # REPL client: connect, send proto messages
└── share/
    ├── proto/                    # Protobuf definitions (package: bm1-proto)
    │   ├── Cargo.toml
    │   ├── build.rs              # prost-build config, #[allow(non_camel_case_types)]
    │   ├── src/lib.rs            # Re-exports message & model modules
    │   └── protos/
    │       ├── message.proto     # CSRpcMsg, CSRpcCmd enum, LoginReq/Resp, AddMoneyReq/Resp
    │       └── model.proto       # PlayerBase, PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerData
    └── protos_build/             # Generated Rust code (DO NOT EDIT)
        ├── message.rs
        └── model.rs
```

## Core API Signatures

### router.rs — routing & handler trait

```rust
pub struct Context { pub session_id: u32, pub player_id: u64 }
pub trait MessageHandler: Send + Sync {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>;
}
pub struct Router { handlers: HashMap<i32, Box<dyn MessageHandler>> }
impl Router {
    pub fn new() -> Self;
    pub fn register(&mut self, cmd: i32, handler: Box<dyn MessageHandler>);
    pub fn dispatch(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>;
}
```

### session.rs — session management

```rust
pub struct Session { pub id: u32, pub connected: bool, pub last_active: Instant, pub player_id: u64 }
pub struct SessionManager { sessions: HashMap<u32, Session>, next_id: u32 }
impl SessionManager {
    pub fn create_session(&mut self) -> u32;
    pub fn reconnect(&mut self, id: u32) -> bool;
    pub fn disconnect(&mut self, id: u32);
    pub fn player_id(&self, id: u32) -> Option<u64>;
    pub fn set_player_id(&mut self, id: u32, player_id: u64);
}
```

### model/player.rs — Player domain model

```rust
pub struct Player { data: PlayerData }
impl Player {
    pub fn data(&self) -> &PlayerData;
    pub fn player_id(&self) -> u64;
    pub fn player_name(&self) -> &str;
    pub fn level(&self) -> u32;
    pub fn add_level(&mut self, delta: u32) -> u32;
    pub fn get_money(&self, money_type: PlayerBagMoneyType) -> u32;
    pub fn add_money(&mut self, money_type: PlayerBagMoneyType, amount: u32) -> u32;
    pub fn sub_money(&mut self, money_type: PlayerBagMoneyType, amount: u32) -> Result<u32, &'static str>;
    pub fn add_item(&mut self, item_id: u32, count: u32) -> u32;
    pub fn sub_item(&mut self, item_id: u32, count: u32) -> Result<u32, &'static str>;
    pub fn gold(&self) -> u32;  pub fn add_gold/sub_gold
    pub fn diamond(&self) -> u32;  pub fn add_diamond/sub_diamond
}
```

### model/player_pool.rs — global player pool

```rust
pub struct PlayerPool { players: HashMap<u64, Player> }
impl PlayerPool {
    pub fn global() -> &'static RwLock<PlayerPool>;  // LazyLock, pre-loaded alice(1) & bob(2)
    pub fn add(&mut self, player: Player) -> Option<Player>;
    pub fn load(&mut self, data: PlayerData) -> Option<Player>;
    pub fn remove(&mut self, player_id: u64) -> Option<Player>;
    pub fn get(&self, player_id: u64) -> Option<&Player>;
    pub fn get_mut(&mut self, player_id: u64) -> Option<&mut Player>;
}
```

## Proto Definitions

### message.proto

```protobuf
message CSRpcMsg {
  CSRpcCmd cmd = 1;
  uint32 seq = 2;
  uint32 session_id = 3;
  oneof payload {
    LoginReq login_req = 14;
    LoginResp login_resp = 15;
    AddMoneyReq add_money_req = 16;
    AddMoneyResp add_money_resp = 17;
  }
}
enum CSRpcCmd {
  CS_RPC_CMD_UNSPECIFIED = 0;
  CS_RPC_CMD_LOGIN_REQ = 3;
  CS_RPC_CMD_LOGIN_RESP = 4;
  CS_RPC_CMD_ADD_MONEY_REQ = 5;
  CS_RPC_CMD_ADD_MONEY_RESP = 6;
}
message LoginReq { uint32 player_id = 1; }
message LoginResp { model.PlayerData player_data = 1; string error_msg = 2; }
message AddMoneyReq { model.PlayerBagMoneyType money_type = 1; uint32 amount = 2; }
message AddMoneyResp { uint32 money_count = 1; string error_msg = 2; }
```

### model.proto

```protobuf
message PlayerBase { uint64 player_id = 1; string player_name = 2; uint32 player_level = 3; }
message PlayerBagItem { uint32 item_id = 1; uint32 item_count = 2; }
enum PlayerBagMoneyType { UNSPECIFIED = 0; GOLD = 1; DIAMOND = 2; }
message PlayerBagMoney { PlayerBagMoneyType money_type = 1; uint32 money_count = 2; }
message PlayerBag { repeated PlayerBagItem items = 1; repeated PlayerBagMoney money = 2; }
message PlayerData { PlayerBase player_base = 1; PlayerBag player_bag = 2; }
```
