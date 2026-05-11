use std::collections::HashMap; // 哈希表，用于存储命令号到处理器的映射

use bm1_proto::bm1::CsRpcMsg; // protobuf RPC 消息类型

/// 路由上下文，在分发时传递给 handler 的环境信息
pub struct Context {
    pub session_id: u32, // 当前连接的会话 ID
}

/// 消息处理器 trait：所有命令处理器必须实现此接口
pub trait MessageHandler: Send + Sync { // Send + Sync 约束确保可跨线程安全使用
    /// 处理消息，返回 Some(resp) 表示需要回复，None 表示不回复
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>;
}

/// 路由器：根据消息的 cmd 字段将消息分发到对应的 handler
pub struct Router {
    handlers: HashMap<i32, Box<dyn MessageHandler>>, // 命令号 → handler 的映射表
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(), // 空映射表
        }
    }

    /// 注册一个命令号对应的处理器
    pub fn register(&mut self, cmd: i32, handler: Box<dyn MessageHandler>) {
        self.handlers.insert(cmd, handler); // 同一个 cmd 注册多次会覆盖
    }

    /// 分发消息：根据 msg.cmd 查找 handler 并调用，返回 handler 的响应
    pub fn dispatch(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        // 查找对应 handler，找到则调用 handle()，未找到或 handler 返回 None 则结果为 None
        self.handlers.get(&msg.cmd).and_then(|h| h.handle(ctx, msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::bm1::CsRpcCmd; // 命令枚举

    // 测试用 handler：原样返回消息（echo）
    struct EchoHandler;

    impl MessageHandler for EchoHandler {
        fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
            Some(CsRpcMsg {
                cmd: msg.cmd,               // 保留原命令号
                seq: msg.seq,               // 保留原序列号
                session_id: ctx.session_id, // 使用上下文中的会话 ID
                payload: None,              // 不带 payload
            })
        }
    }

    #[test]
    fn test_dispatch_known_cmd() {
        let mut router = Router::new();
        router.register(CsRpcCmd::Placeholder as i32, Box::new(EchoHandler)); // 注册 Placeholder 命令

        let ctx = Context { session_id: 1 }; // 上下文：会话 ID 1
        let msg = CsRpcMsg {
            cmd: CsRpcCmd::Placeholder as i32, // Placeholder 命令
            seq: 1,                           // 序列号 1
            session_id: 0,                    // 原始消息中的 session_id（客户端未填）
            payload: None,
        };

        let resp = router.dispatch(&ctx, msg); // 分发消息
        assert!(resp.is_some());              // 有 handler，应该返回 Some
        assert_eq!(resp.unwrap().session_id, 1); // 响应中的 session_id 应为上下文中的值
    }

    #[test]
    fn test_dispatch_unknown_cmd() {
        let router = Router::new(); // 空路由，无注册 handler
        let ctx = Context { session_id: 1 };
        let msg = CsRpcMsg {
            cmd: 999, // 未注册的命令号
            seq: 1,
            session_id: 0,
            payload: None,
        };

        let resp = router.dispatch(&ctx, msg);
        assert!(resp.is_none()); // 无对应 handler，应返回 None
    }
}
