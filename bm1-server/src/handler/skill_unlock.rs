use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, SkillUnlockResp};
use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct SkillUnlockHandler;

impl MessageHandler for SkillUnlockHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let req = match &msg.payload {
            Some(Payload::SkillUnlockReq(r)) => r,
            _ => return None,
        };

        if ctx.player_id == 0 {
            return Some(Self::err_resp(&msg, ctx, "not logged in"));
        }

        let mut pool = PlayerPool::global().write().unwrap();
        let player = match pool.get_mut(ctx.player_id) {
            Some(p) => p,
            None => return Some(Self::err_resp(&msg, ctx, "player not found")),
        };

        match player.unlock_skill(req.skill_id) {
            Ok((skill_id, skill_level)) => {
                let remaining = player.skill_points();
                Some(CsRpcMsg {
                    cmd: CsRpcCmd::SkillUnlockResp as i32,
                    seq: msg.seq,
                    session_id: ctx.session_id,
                    payload: Some(Payload::SkillUnlockResp(SkillUnlockResp {
                        result: 1,
                        error_msg: String::new(),
                        skill_id,
                        skill_level,
                        remaining_skill_points: remaining,
                    })),
                })
            }
            Err(e) => Some(Self::err_resp(&msg, ctx, e)),
        }
    }
}

impl SkillUnlockHandler {
    fn err_resp(msg: &CsRpcMsg, ctx: &Context, err: &str) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::SkillUnlockResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::SkillUnlockResp(SkillUnlockResp {
                result: 0,
                error_msg: err.to_string(),
                ..Default::default()
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::message::SkillUnlockReq;
    use bm1_proto::model::PlayerData;
    use crate::router::Context;

    fn make_req(skill_id: u32) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::SkillUnlockReq as i32,
            seq: 1,
            session_id: 0,
            payload: Some(Payload::SkillUnlockReq(SkillUnlockReq { skill_id })),
        }
    }

    fn logged_in_ctx() -> Context {
        Context { player_id: 102, session_id: 100 }
    }

    fn not_logged_in_ctx() -> Context {
        Context { player_id: 0, session_id: 0 }
    }

    fn setup_player_with_points(points: u32) {
        let mut pool = PlayerPool::global().write().unwrap();
        pool.load(PlayerData {
            player_base: Some(bm1_proto::model::PlayerBase {
                player_id: 102,
                player_name: "test".into(),
                player_level: 1,
            }),
            player_bag: None,
            player_skill: Some(bm1_proto::model::PlayerSkillData {
                skill_points: points,
                skills: vec![],
            }),
        });
    }

    #[test]
    fn test_not_logged_in() {
        let handler = SkillUnlockHandler;
        let msg = make_req(1);
        let resp = handler.handle(&not_logged_in_ctx(), msg).unwrap();
        assert_eq!(resp.cmd, CsRpcCmd::SkillUnlockResp as i32);
        match resp.payload {
            Some(Payload::SkillUnlockResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(!r.error_msg.is_empty());
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_unlock_success() {
        setup_player_with_points(5);
        let handler = SkillUnlockHandler;
        let msg = make_req(100);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::SkillUnlockResp(r)) => {
                assert_eq!(r.result, 1);
                assert_eq!(r.skill_id, 100);
                assert_eq!(r.skill_level, 1);
                assert_eq!(r.remaining_skill_points, 4);
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_unlock_duplicate() {
        setup_player_with_points(5);
        let handler = SkillUnlockHandler;
        let _ = handler.handle(&logged_in_ctx(), make_req(200));
        let resp = handler.handle(&logged_in_ctx(), make_req(200)).unwrap();
        match resp.payload {
            Some(Payload::SkillUnlockResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("already unlocked"));
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_unlock_insufficient_points() {
        setup_player_with_points(0);
        let handler = SkillUnlockHandler;
        let msg = make_req(300);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::SkillUnlockResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("insufficient"));
            }
            _ => panic!("unexpected payload"),
        }
    }
}
