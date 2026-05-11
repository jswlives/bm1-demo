# bm1-server 高性能游戏服务器 Demo 设计

## 概述

使用 Rust 编写的高性能 TCP 服务器 demo，协议使用 protobuf，支持多客户端连接、session 管理、断线重连、cmd 路由分发、双向心跳。

## 项目结构（Cargo Workspace）

```
bm1-demo/
├── Cargo.toml              # workspace 根
├── bm1-server/             # 服务器
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # 入口，启动 tokio runtime
│       ├── server.rs       # TCP listener，接受连接
│       ├── session.rs      # Session 管理（连接、断线重连、收发）
│       ├── router.rs       # CSRpcCmd 枚举 + Handler trait + 路由分发
│       └── handler/        # 各 cmd handler 实现
│           ├── mod.rs
│           └── placeholder.rs  # 占位 cmd handler
├── client/                 # 模拟客户端
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── share/
    └── proto/              # protobuf 定义
        ├── Cargo.toml      # bm1-proto crate
        ├── build.rs        # prost 编译 proto
        ├── src/
        │   └── lib.rs
        └── protos/
            └── message.proto
```

## 协议设计

### 帧格式（TCP 粘包处理）

```
[4 bytes: len (big-endian u32)] [len bytes: protobuf body]
```

### message.proto

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

## Session 管理

- 每个 TCP 连接创建一个 `Session`，分配唯一 `session_id`（自增 u32）
- `SessionManager`（`Arc<Mutex<HashMap<u32, Session>>`）维护所有活跃 session
- 断线时：Session 标记为 disconnected，保留在 manager 中（等待重连超时后清理）
- 重连时：客户端发送携带 `session_id` 的 CSRpcMsg，服务器恢复对应 session，替换底层 TCP stream
- 新连接 vs 重连：`session_id == 0` 为新连接，非 0 为重连请求

## 双向心跳

- 客户端和服务端都定时发送 HeartbeatReq，对方回复 HeartbeatResp
- 服务端：每个 Session spawn 心跳检测任务，每 10 秒检查最后收到消息的时间
- 超时阈值：30 秒未收到任何消息则断开连接
- 收到任何消息（包括业务消息）都刷新最后活跃时间

## 路由分发

```rust
enum CSRpcCmd { ... }

trait MessageHandler: Send + Sync {
    fn cmd(&self) -> CSRpcCmd;
    fn handle(&self, ctx: &Context, msg: CSRpcMsg) -> Result<CSRpcMsg>;
}

struct Router {
    handlers: HashMap<CSRpcCmd, Box<dyn MessageHandler>>,
}

impl Router {
    fn dispatch(&self, ctx: &Context, msg: CSRpcMsg) -> Result<CSRpcMsg> {
        let handler = self.handlers.get(&msg.cmd)?;
        handler.handle(ctx, msg)
    }
}
```

## 关键依赖

| crate | 用途 |
|-------|------|
| tokio | 异步运行时 + TCP |
| prost / prost-types | protobuf 编译与编解码 |
| bytes | 高效字节缓冲区 |
| anyhow | 错误处理 |

## 数据流

```
Client → TCP → Server accept → spawn Session task
  → 读取 4 bytes len → 读取 len bytes body
  → CSRpcMsg::decode(body) → Router::dispatch(msg)
  → Handler::handle() → 返回 CSRpcMsg
  → CSRpcMsg::encode() → 写入 [len][body] → TCP → Client
```
