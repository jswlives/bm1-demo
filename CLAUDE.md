# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Build entire workspace
cargo build -p bm1-server            # Build server only
cargo build -p bm1-client            # Build client only
cargo build -p bm1-proto             # Rebuild proto types after proto changes
cargo test                           # Run all tests
cargo test -p bm1-server             # Run server tests only
cargo test -p bm1-server -- test_name  # Run a single test
cargo run -p bm1-server              # Start server (listens on 0.0.0.0:8080)
cargo run -p bm1-client              # Start interactive client
```

Requires `protoc` installed (`brew install protobuf` on macOS, `apt install protobuf-compiler` on Ubuntu).

## Architecture

Cargo workspace with three crates: `bm1-server`, `bm1-client`, `bm1-proto`.

```
Client TCP → Server accept → spawn task per connection
  → codec::read_frame [4-byte len + protobuf body]
  → Router::dispatch by msg.cmd (i32)
  → Handler::handle → returns CsRpcMsg
  → codec::write_frame → TCP → Client
```

### Key Design Decisions

- **prost type names**: `CSRpcMsg` → `CsRpcMsg`, `CSRpcCmd` → `CsRpcCmd`. The oneof `payload` generates `cs_rpc_msg::Payload` enum. The `cmd` field is `i32`, not the enum type.
- **Session lifecycle**: `session_id == 0` creates new, non-zero attempts reconnect. `SessionManager` behind `Arc<Mutex<...>>`.
- **Handler trait**: `MessageHandler::handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>`. Return `Some` to send response, `None` to drop.

### Registered Commands

| CSRpcCmd | i32 | Handler |
|---|---|---|
| LoginReq | 3 | LoginHandler |
| LoginResp | 4 | — |
| AddMoneyReq | 5 | AddMoneyHandler |
| AddMoneyResp | 6 | — |
| PlayerDataNotify | 7 | (server-push, no handler) |
| SkillUnlockReq | 8 | SkillUnlockHandler |
| SkillUnlockResp | 9 | — |
| SkillUpgradeReq | 10 | SkillUpgradeHandler |
| SkillUpgradeResp | 11 | — |
| BuyEquipReq | 12 | BuyEquipHandler |
| BuyEquipResp | 13 | — |
| UpgradeEquipReq | 14 | UpgradeEquipHandler |
| UpgradeEquipResp | 15 | — |

Next available cmd: **16**. Next available oneof field number: **27**.

### Adding a New Command

1. Add enum value to `CSRpcCmd` and message/oneof fields to `share/proto/protos/message.proto`
2. If needed, add new data types to `share/proto/protos/model.proto`
3. `cargo build -p bm1-proto` to regenerate types
4. Create handler in `bm1-server/src/handler/` implementing `MessageHandler`
5. Export in `handler/mod.rs`, register in `server.rs` `build_router()`

## Reference Docs

When you need detailed code structure or API signatures, read these docs instead of scanning the codebase:

- **`docs/code-structure.md`** — Full file tree with descriptions and core API signatures
- **`docs/context-optimization-guide.md`** — How to keep context efficient
