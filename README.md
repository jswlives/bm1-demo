# bm1-demo

Rust 高性能游戏服务器 Demo，使用 TCP + Protobuf 协议，支持多客户端连接、Session 管理、断线重连、Cmd 路由分发、双向心跳。

## 项目结构

```
bm1-demo/
├── Cargo.toml              # Workspace 根配置
├── bm1-server/             # 服务器
│   └── src/
│       ├── main.rs         # 入口
│       ├── server.rs       # TCP 监听、连接处理
│       ├── session.rs      # Session 管理
│       ├── codec.rs        # 帧编解码
│       ├── router.rs       # Cmd 路由 + Handler trait
│       └── handler/        # 各 Cmd 的处理实现
│           ├── mod.rs
│           ├── placeholder.rs
│           └── heartbeat.rs
├── client/                 # 模拟客户端
│   └── src/main.rs
└── share/
    └── proto/              # 共享 Protobuf 定义
        ├── build.rs
        ├── src/lib.rs
        └── protos/
            └── message.proto
```

## 初始化

需要安装 Rust 和 protoc：

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 protoc（macOS）
brew install protobuf

# 安装 protoc（Ubuntu）
sudo apt install -y protobuf-compiler

# 验证
rustc --version
protoc --version
```

克隆项目后，依赖会自动下载：

```bash
git clone <repo-url> bm1-demo
cd bm1-demo
cargo build
```

## 新增协议

以新增一个 `Chat` 协议为例，分三步：

### 1. 定义 Proto

编辑 `share/proto/protos/message.proto`，添加三处内容：

```protobuf
// 1. 在 CSRpcCmd 枚举中添加新值
enum CSRpcCmd {
  CS_RPC_CMD_UNSPECIFIED = 0;
  CS_RPC_CMD_PLACEHOLDER = 1;
  CS_RPC_CMD_HEARTBEAT = 2;
  CS_RPC_CMD_CHAT = 3;              // 新增
}

// 2. 在 CSRpcMsg 的 oneof payload 中添加字段
oneof payload {
  // ... 已有字段 ...
  ChatReq chat_req = 14;            // 新增，field number 从 14 开始递增
  ChatResp chat_resp = 15;
}

// 3. 定义 Request/Response message
message ChatReq {
  string content = 1;
}

message ChatResp {
  string content = 1;
}
```

修改后重新编译生成 Rust 类型：

```bash
cargo build -p bm1-proto
```

### 2. 实现 Handler

创建 `bm1-server/src/handler/chat.rs`：

```rust
use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, ChatResp};
use bm1_proto::bm1::cs_rpc_msg::Payload;

use crate::router::{Context, MessageHandler};

pub struct ChatHandler;

impl MessageHandler for ChatHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let content = match &msg.payload {
            Some(Payload::ChatReq(req)) => &req.content,
            _ => return None,
        };

        Some(CsRpcMsg {
            cmd: CsRpcCmd::Chat as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::ChatResp(ChatResp {
                content: content.clone(),
            })),
        })
    }
}
```

### 3. 注册 Handler

在 `bm1-server/src/handler/mod.rs` 中导出：

```rust
mod placeholder;
mod heartbeat;
mod chat;                              // 新增

pub use placeholder::PlaceholderHandler;
pub use heartbeat::HeartbeatHandler;
pub use chat::ChatHandler;             // 新增
```

在 `bm1-server/src/server.rs` 的 `build_router()` 中注册：

```rust
fn build_router() -> Router {
    let mut router = Router::new();
    router.register(CsRpcCmd::Placeholder as i32, Box::new(PlaceholderHandler));
    router.register(CsRpcCmd::Heartbeat as i32, Box::new(HeartbeatHandler));
    router.register(CsRpcCmd::Chat as i32, Box::new(ChatHandler));    // 新增
    router
}
```

## 构建

```bash
# 构建全部
cargo build

# 构建服务器
cargo build -p bm1-server

# 构建客户端
cargo build -p bm1-client

# Release 构建
cargo build --release
```

## 运行测试

### 启动服务器

```bash
cargo run -p bm1-server
```

服务器默认监听 `0.0.0.0:8080`，输出：

```
server listening on 0.0.0.0:8080
```

### 启动客户端

另开一个终端：

```bash
cargo run -p bm1-client
```

客户端启动后进入交互模式，显示菜单：

```
connected to server

=== session_id: 0 ===
[1] Placeholder  -  发送测试消息
[2] Heartbeat    -  发送心跳
[3] Reconnect    -  断线重连
[0] Exit         -  退出

选择操作:
```

#### 操作说明

| 选项 | 说明 |
|------|------|
| `1` | 发送 PlaceholderReq，需输入消息内容，服务器 echo 回复 |
| `2` | 发送 HeartbeatReq，服务器返回时间戳 |
| `3` | 断开当前连接并重新连接，保留 session_id 验证断线重连 |
| `0` | 退出客户端 |

#### 交互示例

```
选择操作: 1
输入消息内容: hello
>>> 发送 PlaceholderReq: hello
<<< 收到 PlaceholderResp: echo: hello

=== session_id: 1 ===
[1] Placeholder  -  发送测试消息
[2] Heartbeat    -  发送心跳
[3] Reconnect    -  断线重连
[0] Exit         -  退出

选择操作: 2
>>> 发送 HeartbeatReq: timestamp=1778148486969
<<< 收到 HeartbeatResp: timestamp=1778148486969

选择操作: 3
--- 断开连接 ---
--- 重新连接 ---
connected to server

选择操作: 1
输入消息内容: after reconnect
>>> 发送 PlaceholderReq: after reconnect
<<< 收到 PlaceholderResp: echo: after reconnect

选择操作: 0
bye!
```

对应服务器输出：

```
server listening on 0.0.0.0:8080
connection from 127.0.0.1:XXXXX
session 1 established
session 1 disconnected
connection from 127.0.0.1:YYYYY
session 1 established
session 1 disconnected
```

### 运行单元测试

```bash
cargo test
```

## 协议格式

TCP 帧格式（解决粘包）：

```
[4 bytes: body长度 (big-endian u32)] [body: protobuf 编码的 CSRpcMsg]
```
