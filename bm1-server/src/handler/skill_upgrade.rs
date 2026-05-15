use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, SkillUpgradeResp};
use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct SkillUpgradeHandler;

impl MessageHandler for SkillUpgradeHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let req = match &msg.payload {
            Some(Payload::SkillUpgradeReq(r)) => r,
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

        match player.upgrade_skill(req.skill_id) {
            Ok((skill_id, skill_level)) => {
                let remaining = player.skill_points();
                Some(CsRpcMsg {
                    cmd: CsRpcCmd::SkillUpgradeResp as i32,
                    seq: msg.seq,
                    session_id: ctx.session_id,
                    payload: Some(Payload::SkillUpgradeResp(SkillUpgradeResp {
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

impl SkillUpgradeHandler {
    fn err_resp(msg: &CsRpcMsg, ctx: &Context, err: &str) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::SkillUpgradeResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::SkillUpgradeResp(SkillUpgradeResp {
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
    use bm1_proto::message::SkillUpgradeReq;
    use bm1_proto::model::PlayerData;
    use crate::router::Context;

    fn make_req(skill_id: u32) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::SkillUpgradeReq as i32,
            seq: 1,
            session_id: 0,
            payload: Some(Payload::SkillUpgradeReq(SkillUpgradeReq { skill_id })),
        }
    }

    fn logged_in_ctx() -> Context {
        Context { player_id: 103, session_id: 100 }
    }

    fn not_logged_in_ctx() -> Context {
        Context { player_id: 0, session_id: 0 }
    }

    fn setup_player_with_skill() {
        let mut pool = PlayerPool::global().write().unwrap();
        pool.load(PlayerData {
            player_base: Some(bm1_proto::model::PlayerBase {
                player_id: 103,
                player_name: "test".into(),
                player_level: 1,
            }),
            player_bag: None,
            player_skill: Some(bm1_proto::model::PlayerSkillData {
                skill_points: 5,
                skills: vec![bm1_proto::model::PlayerSkill { skill_id: 100, skill_level: 1 }],
            }),
        });
    }

    #[test]
    fn test_not_logged_in() {
        let handler = SkillUpgradeHandler;
        let msg = make_req(100);
        let resp = handler.handle(&not_logged_in_ctx(), msg).unwrap();
        assert_eq!(resp.cmd, CsRpcCmd::SkillUpgradeResp as i32);
        match resp.payload {
            Some(Payload::SkillUpgradeResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(!r.error_msg.is_empty());
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_success() {
        setup_player_with_skill();
        let handler = SkillUpgradeHandler;
        let msg = make_req(100);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::SkillUpgradeResp(r)) => {
                assert_eq!(r.result, 1);
                assert_eq!(r.skill_id, 100);
                assert_eq!(r.skill_level, 2);
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_not_unlocked() {
        setup_player_with_skill();
        let handler = SkillUpgradeHandler;
        let msg = make_req(999);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::SkillUpgradeResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("not unlocked"));
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_insufficient_points() {
        setup_player_with_skill();
        // Exhaust skill points by upgrading multiple times
        let handler = SkillUpgradeHandler;
        for _ in 0..10 {
            let _ = handler.handle(&logged_in_ctx(), make_req(100));
        }
        let resp = handler.handle(&logged_in_ctx(), make_req(100)).unwrap();
        match resp.payload {
            Some(Payload::SkillUpgradeResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("insufficient"));
            }
            _ => panic!("unexpected payload"),
        }
    }
}
