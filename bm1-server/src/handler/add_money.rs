use bm1_proto::message::cs_rpc_msg::Payload;
use bm1_proto::message::{AddMoneyResp, CsRpcCmd, CsRpcMsg};
use bm1_proto::model::PlayerBagMoneyType;

use crate::model::player_pool::PlayerPool;
use crate::router::{Context, MessageHandler};

pub struct AddMoneyHandler;

impl MessageHandler for AddMoneyHandler {
    fn handle(&self, ctx: &Context, msg: CsRpcMsg) -> Option<CsRpcMsg> {
        if ctx.player_id == 0 {
            return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "not logged in".into(),
                })),
            });
        }

        let (money_type, amount) = match &msg.payload {
            Some(Payload::AddMoneyReq(req)) => (req.money_type, req.amount),
            _ => return None,
        };

        let money_type_enum = match money_type {
            0 => PlayerBagMoneyType::Unspecified,
            1 => PlayerBagMoneyType::Gold,
            2 => PlayerBagMoneyType::Diamond,
            _ => PlayerBagMoneyType::Unspecified,
        };

        if money_type_enum == PlayerBagMoneyType::Unspecified {
            return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "invalid money type".into(),
                })),
            });
        }

        let mut pool = PlayerPool::global().write().unwrap();
        let player = match pool.get_mut(ctx.player_id) {
            Some(p) => p,
            None => return Some(CsRpcMsg {
                cmd: CsRpcCmd::AddMoneyResp as i32,
                seq: msg.seq,
                session_id: ctx.session_id,
                payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                    money_count: 0,
                    error_msg: "player not found".into(),
                })),
            }),
        };

        let new_count = player.add_money(money_type_enum, amount);

        Some(CsRpcMsg {
            cmd: CsRpcCmd::AddMoneyResp as i32,
            seq: msg.seq,
            session_id: ctx.session_id,
            payload: Some(Payload::AddMoneyResp(AddMoneyResp {
                money_count: new_count,
                error_msg: String::new(),
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::message::{AddMoneyReq, CsRpcCmd};
    use bm1_proto::model::PlayerBagMoneyType;

    fn make_add_money_msg(money_type: i32, amount: u32) -> CsRpcMsg {
        CsRpcMsg {
            cmd: CsRpcCmd::AddMoneyReq as i32,
            seq: 1,
            session_id: 1,
            payload: Some(Payload::AddMoneyReq(AddMoneyReq {
                money_type,
                amount,
            })),
        }
    }

    #[test]
    fn test_add_money_not_logged_in() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 0 };
        let msg = make_add_money_msg(PlayerBagMoneyType::Gold as i32, 100);

        let resp = handler.handle(&ctx, msg).unwrap();
        assert_eq!(resp.cmd, CsRpcCmd::AddMoneyResp as i32);
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert_eq!(r.money_count, 0);
            assert!(!r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }
    }

    #[test]
    fn test_add_money_gold() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 1 };
        let msg = make_add_money_msg(PlayerBagMoneyType::Gold as i32, 50);

        let resp = handler.handle(&ctx, msg).unwrap();
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert_eq!(r.money_count, 1050); // alice starts with 1000 gold
            assert!(r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }

        // Restore
        PlayerPool::global().write().unwrap().get_mut(1).unwrap().sub_gold(50).unwrap();
    }

    #[test]
    fn test_add_money_invalid_type() {
        let handler = AddMoneyHandler;
        let ctx = Context { session_id: 1, player_id: 1 };
        let msg = make_add_money_msg(0, 100); // Unspecified

        let resp = handler.handle(&ctx, msg).unwrap();
        if let Some(Payload::AddMoneyResp(r)) = resp.payload {
            assert!(!r.error_msg.is_empty());
        } else {
            panic!("expected AddMoneyResp");
        }
    }
}
