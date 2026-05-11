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

### Data Flow

```
Client TCP → Server accept → spawn task per connection
  → codec::read_frame [4-byte len + protobuf body]
  → Router::dispatch by msg.cmd (i32)
  → Handler::handle → returns CsRpcMsg
  → codec::write_frame → TCP → Client
```

### Key Design Decisions

- **prost type names**: prost converts proto names — `CSRpcMsg` → `CsRpcMsg`, `CSRpcCmd` → `CsRpcCmd`. The oneof `payload` generates `cs_rpc_msg::Payload` enum with variants like `Payload::PlaceholderReq(...)`. The `cmd` field on `CsRpcMsg` is `i32`, not the enum type. `build.rs` adds `#[allow(non_camel_case_types)]` to suppress warnings.
- **Session lifecycle**: First message from client determines session — `session_id == 0` creates new, non-zero attempts reconnect. `SessionManager` tracks sessions in `HashMap<u32, Session>` behind `Arc<Mutex<...>>`.
- **Connection handling**: TCP stream split into read/write halves. Write side driven by `mpsc::channel` so both handler responses and heartbeat sends can write without contention. Heartbeat sends every 10s, 30s idle timeout disconnects.
- **Handler trait**: `MessageHandler::handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>`. Return `Some` to send response, `None` to drop. Handlers are `Send + Sync` trait objects in `Router`'s `HashMap<i32, Box<dyn MessageHandler>>`.

### Adding a New Command

1. Add enum value to `CSRpcCmd` and message/oneof fields to `share/proto/protos/message.proto` (field numbers start at 14 for new payload fields)
2. `cargo build -p bm1-proto` to regenerate types
3. Create handler in `bm1-server/src/handler/` implementing `MessageHandler`
4. Export in `handler/mod.rs`, register in `server.rs` `build_router()`
