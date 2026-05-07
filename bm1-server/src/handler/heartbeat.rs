use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, HeartbeatResp};
use bm1_proto::bm1::cs_rpc_msg::Payload;

use crate::router::{Context, MessageHandler};

pub struct HeartbeatHandler;

impl MessageHandler for HeartbeatHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let timestamp = match &msg.payload {
            Some(Payload::HeartbeatReq(req)) => req.timestamp,
            _ => 0,
        };

        Some(CsRpcMsg {
            cmd: CsRpcCmd::Heartbeat as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::HeartbeatResp(HeartbeatResp {
                timestamp,
            })),
        })
    }
}
