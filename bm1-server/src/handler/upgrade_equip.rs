use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{CsRpcCmd, CsRpcMsg, UpgradeEquipResp};
use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct UpgradeEquipHandler;

impl MessageHandler for UpgradeEquipHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        let req = match &msg.payload {
            Some(Payload::UpgradeEquipReq(r)) => r,
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

        match player.upgrade_equip(req.equip_id) {
            Ok((equip_id, equip_level)) => {
                Some(CsRpcMsg {
                    cmd: CsRpcCmd::UpgradeEquipResp as i32,
                    seq: msg.seq,
                    session_id: ctx.session_id,
                    payload: Some(Payload::UpgradeEquipResp(UpgradeEquipResp {
                        result: 1,
                        error_msg: String::new(),
                        equip_id,
                        equip_level,
                    })),
                })
            }
            Err(e) => Some(Self::err_resp(&msg, ctx, e)),
        }
    }
}

impl UpgradeEquipHandler {
    fn err_resp(msg: &CsRpcMsg, ctx: &Context, err: &str) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::UpgradeEquipResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::UpgradeEquipResp(UpgradeEquipResp {
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
    use bm1_proto::message::UpgradeEquipReq;
    use bm1_proto::model::{PlayerBag, PlayerBagMoney, PlayerBagMoneyType, PlayerBase, PlayerData, PlayerEquip, PlayerEquipData};
    use crate::router::Context;

    fn make_req(equip_id: u32) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::UpgradeEquipReq as i32,
            seq: 1,
            session_id: 0,
            payload: Some(Payload::UpgradeEquipReq(UpgradeEquipReq { equip_id })),
        }
    }

    fn logged_in_ctx() -> Context {
        Context { player_id: 202, session_id: 100 }
    }

    fn not_logged_in_ctx() -> Context {
        Context { player_id: 0, session_id: 0 }
    }

    fn setup_player_with_equip() {
        let mut pool = PlayerPool::global().write().unwrap();
        pool.load(PlayerData {
            player_base: Some(PlayerBase {
                player_id: 202,
                player_name: "test".into(),
                player_level: 1,
            }),
            player_bag: Some(PlayerBag {
                items: vec![],
                money: vec![PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Gold as i32,
                    money_count: 500,
                }],
            }),
            player_skill: None,
            player_equip: Some(PlayerEquipData {
                equips: vec![PlayerEquip { equip_id: 1001, equip_level: 1 }],
            }),
        });
    }

    #[test]
    fn test_not_logged_in() {
        let handler = UpgradeEquipHandler;
        let msg = make_req(1001);
        let resp = handler.handle(&not_logged_in_ctx(), msg).unwrap();
        assert_eq!(resp.cmd, CsRpcCmd::UpgradeEquipResp as i32);
        match resp.payload {
            Some(Payload::UpgradeEquipResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(!r.error_msg.is_empty());
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_success() {
        setup_player_with_equip();
        let handler = UpgradeEquipHandler;
        let msg = make_req(1001);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::UpgradeEquipResp(r)) => {
                assert_eq!(r.result, 1);
                assert_eq!(r.equip_id, 1001);
                assert_eq!(r.equip_level, 2);
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_not_owned() {
        setup_player_with_equip();
        let handler = UpgradeEquipHandler;
        let msg = make_req(9999);
        let resp = handler.handle(&logged_in_ctx(), msg).unwrap();
        match resp.payload {
            Some(Payload::UpgradeEquipResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("not owned"));
            }
            _ => panic!("unexpected payload"),
        }
    }

    #[test]
    fn test_upgrade_insufficient_gold() {
        setup_player_with_equip();
        let handler = UpgradeEquipHandler;
        // Drain gold by upgrading multiple times
        for _ in 0..6 {
            let _ = handler.handle(&logged_in_ctx(), make_req(1001));
        }
        let resp = handler.handle(&logged_in_ctx(), make_req(1001)).unwrap();
        match resp.payload {
            Some(Payload::UpgradeEquipResp(r)) => {
                assert_eq!(r.result, 0);
                assert!(r.error_msg.contains("insufficient"));
            }
            _ => panic!("unexpected payload"),
        }
    }
}
