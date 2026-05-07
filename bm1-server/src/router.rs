use std::collections::HashMap;

use bm1_proto::bm1::CsRpcMsg;

pub struct Context {
    pub session_id: u32,
}

pub trait MessageHandler: Send + Sync {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg>;
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

    pub fn dispatch(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        self.handlers.get(&msg.cmd).and_then(|h| h.handle(ctx, msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::bm1::CsRpcCmd;

    struct EchoHandler;

    impl MessageHandler for EchoHandler {
        fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
            Some(CsRpcMsg {
                cmd: msg.cmd,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: None,
            })
        }
    }

    #[test]
    fn test_dispatch_known_cmd() {
        let mut router = Router::new();
        router.register(CsRpcCmd::Placeholder as i32, Box::new(EchoHandler));

        let ctx = Context { session_id: 1 };
        let msg = CsRpcMsg {
            cmd: CsRpcCmd::Placeholder as i32,
            seq: 1,
            session_id: 0,
            payload: None,
        };

        let resp = router.dispatch(&ctx, msg);
        assert!(resp.is_some());
        assert_eq!(resp.unwrap().session_id, 1);
    }

    #[test]
    fn test_dispatch_unknown_cmd() {
        let router = Router::new();
        let ctx = Context { session_id: 1 };
        let msg = CsRpcMsg {
            cmd: 999,
            seq: 1,
            session_id: 0,
            payload: None,
        };

        let resp = router.dispatch(&ctx, msg);
        assert!(resp.is_none());
    }
}
