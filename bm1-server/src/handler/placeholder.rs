use bm1_proto::bm1::{CsRpcCmd, CsRpcMsg, PlaceholderResp};
use bm1_proto::bm1::cs_rpc_msg::Payload;

use crate::router::{Context, MessageHandler};

pub struct PlaceholderHandler;

impl MessageHandler for PlaceholderHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let req_msg = match &msg.payload {
            Some(Payload::PlaceholderReq(req)) => &req.msg,
            _ => return None,
        };

        Some(CsRpcMsg {
            cmd: CsRpcCmd::Placeholder as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::PlaceholderResp(PlaceholderResp {
                msg: format!("echo: {}", req_msg),
            })),
        })
    }
}
