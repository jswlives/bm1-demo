use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, LoginResp};

use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct LoginHandler;

impl MessageHandler for LoginHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let player_id = match &msg.payload {
            Some(Payload::LoginReq(req)) => req.player_id,
            _ => return None,
        };

        let (player_data, error_msg) = match PlayerPool::global().get(player_id as u64) {
            Some(player) => (Some(player.data().clone()), String::new()),
            None => (None, format!("player {} not found", player_id)),
        };

        Some(CsRpcMsg {
            cmd: CsRpcCmd::LoginResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::LoginResp(LoginResp {
                player_data,
                error_msg,
            })),
        })
    }
}
