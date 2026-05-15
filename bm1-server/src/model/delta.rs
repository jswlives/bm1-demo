use bm1_proto::model::{
    DeltaOp, PlayerBagDelta, PlayerBagItemDelta, PlayerBagMoneyDelta,
    PlayerBaseDelta, PlayerData, PlayerDataDelta, PlayerSkillDataDelta, PlayerSkillDelta,
};

pub fn diff_player_data(before: &PlayerData, after: &PlayerData) -> Option<PlayerDataDelta> {
    let base = diff_base(before, after);
    let bag = diff_bag(before, after);
    let skill = diff_skill(before, after);

    if base.is_none() && bag.is_none() && skill.is_none() {
        return None;
    }

    Some(PlayerDataDelta { base, bag, skill })
}

// Only diffs player_level; player_id and player_name are immutable at runtime.
fn diff_base(before: &PlayerData, after: &PlayerData) -> Option<PlayerBaseDelta> {
    let before_level = before.player_base.as_ref().map(|b| b.player_level).unwrap_or(0);
    let after_level = after.player_base.as_ref().map(|b| b.player_level).unwrap_or(0);

    if before_level == after_level {
        return None;
    }

    Some(PlayerBaseDelta {
        player_level: Some(after_level),
    })
}

fn diff_bag(before: &PlayerData, after: &PlayerData) -> Option<PlayerBagDelta> {
    let money_changes = diff_money(before, after);
    let item_changes = diff_items(before, after);

    if money_changes.is_empty() && item_changes.is_empty() {
        return None;
    }

    Some(PlayerBagDelta {
        money_changes,
        item_changes,
    })
}

fn diff_money(before: &PlayerData, after: &PlayerData) -> Vec<PlayerBagMoneyDelta> {
    let before_money = before.player_bag.as_ref().map(|b| &b.money).unwrap_or(&EMPTY_MONEY);
    let after_money = after.player_bag.as_ref().map(|b| &b.money).unwrap_or(&EMPTY_MONEY);

    let mut changes = Vec::new();

    for am in after_money {
        let before_count = before_money
            .iter()
            .find(|m| m.money_type == am.money_type)
            .map(|m| m.money_count)
            .unwrap_or(0);

        if am.money_count != before_count {
            changes.push(PlayerBagMoneyDelta {
                money_type: am.money_type,
                money_count: am.money_count,
            });
        }
    }

    // Money types removed entirely (count went to 0 or entry removed)
    for bm in before_money {
        let exists_in_after = after_money
            .iter()
            .any(|m| m.money_type == bm.money_type);
        if !exists_in_after && bm.money_count > 0 {
            changes.push(PlayerBagMoneyDelta {
                money_type: bm.money_type,
                money_count: 0,
            });
        }
    }

    changes
}

fn diff_items(before: &PlayerData, after: &PlayerData) -> Vec<PlayerBagItemDelta> {
    let before_items = before.player_bag.as_ref().map(|b| &b.items).unwrap_or(&EMPTY_ITEMS);
    let after_items = after.player_bag.as_ref().map(|b| &b.items).unwrap_or(&EMPTY_ITEMS);

    let mut changes = Vec::new();

    // Items in after that are new or changed → UPSERT
    for ai in after_items {
        let before_count = before_items
            .iter()
            .find(|i| i.item_id == ai.item_id)
            .map(|i| i.item_count)
            .unwrap_or(0);

        if ai.item_count != before_count {
            changes.push(PlayerBagItemDelta {
                op: DeltaOp::Upsert as i32,
                item_id: ai.item_id,
                item_count: ai.item_count,
            });
        }
    }

    // Items in before but not in after → DELETE
    for bi in before_items {
        let exists_in_after = after_items.iter().any(|i| i.item_id == bi.item_id);
        if !exists_in_after {
            changes.push(PlayerBagItemDelta {
                op: DeltaOp::Delete as i32,
                item_id: bi.item_id,
                item_count: 0,
            });
        }
    }

    changes
}

static EMPTY_MONEY: Vec<bm1_proto::model::PlayerBagMoney> = Vec::new();
static EMPTY_ITEMS: Vec<bm1_proto::model::PlayerBagItem> = Vec::new();

fn diff_skill(before: &PlayerData, after: &PlayerData) -> Option<PlayerSkillDataDelta> {
    let before_skill = before.player_skill.as_ref();
    let after_skill = after.player_skill.as_ref();

    let before_points = before_skill.map(|s| s.skill_points).unwrap_or(0);
    let after_points = after_skill.map(|s| s.skill_points).unwrap_or(0);

    let before_skills = before_skill.map(|s| &s.skills).unwrap_or(&EMPTY_SKILLS);
    let after_skills = after_skill.map(|s| &s.skills).unwrap_or(&EMPTY_SKILLS);

    let points_changed = before_points != after_points;

    let mut skill_changes = Vec::new();

    for as_ in after_skills {
        let before_level = before_skills
            .iter()
            .find(|s| s.skill_id == as_.skill_id)
            .map(|s| s.skill_level)
            .unwrap_or(0);

        if as_.skill_level != before_level {
            skill_changes.push(PlayerSkillDelta {
                op: DeltaOp::Upsert as i32,
                skill_id: as_.skill_id,
                skill_level: as_.skill_level,
            });
        }
    }

    for bs in before_skills {
        let exists_in_after = after_skills.iter().any(|s| s.skill_id == bs.skill_id);
        if !exists_in_after {
            skill_changes.push(PlayerSkillDelta {
                op: DeltaOp::Delete as i32,
                skill_id: bs.skill_id,
                skill_level: 0,
            });
        }
    }

    if !points_changed && skill_changes.is_empty() {
        return None;
    }

    Some(PlayerSkillDataDelta {
        skill_points: if points_changed { Some(after_points) } else { None },
        skill_changes,
    })
}

static EMPTY_SKILLS: Vec<bm1_proto::model::PlayerSkill> = Vec::new();

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::model::{PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerBagMoneyType, PlayerBase};

    fn make_player_data(level: u32, gold: u32, diamond: u32, items: Vec<(u32, u32)>) -> PlayerData {
        PlayerData {
            player_base: Some(PlayerBase {
                player_id: 1,
                player_name: "test".into(),
                player_level: level,
            }),
            player_bag: Some(PlayerBag {
                items: items.into_iter().map(|(id, count)| PlayerBagItem { item_id: id, item_count: count }).collect(),
                money: vec![
                    PlayerBagMoney { money_type: PlayerBagMoneyType::Gold as i32, money_count: gold },
                    PlayerBagMoney { money_type: PlayerBagMoneyType::Diamond as i32, money_count: diamond },
                ],
            }),
            player_skill: None,
        }
    }

    #[test]
    fn test_no_change_returns_none() {
        let data = make_player_data(1, 100, 50, vec![]);
        assert!(diff_player_data(&data, &data).is_none());
    }

    #[test]
    fn test_level_change() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_base.as_mut().unwrap().player_level = 3;

        let delta = diff_player_data(&before, &after).unwrap();
        assert_eq!(delta.base.unwrap().player_level, Some(3));
        assert!(delta.bag.is_none());
    }

    #[test]
    fn test_money_change() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().money[0].money_count = 200;

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.money_changes.len(), 1);
        assert_eq!(bag.money_changes[0].money_type, PlayerBagMoneyType::Gold as i32);
        assert_eq!(bag.money_changes[0].money_count, 200);
    }

    #[test]
    fn test_item_added() {
        let before = make_player_data(1, 100, 50, vec![]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().items.push(PlayerBagItem { item_id: 1001, item_count: 5 });

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Upsert as i32);
        assert_eq!(bag.item_changes[0].item_id, 1001);
        assert_eq!(bag.item_changes[0].item_count, 5);
    }

    #[test]
    fn test_item_removed() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let after = make_player_data(1, 100, 50, vec![]);

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Delete as i32);
        assert_eq!(bag.item_changes[0].item_id, 1001);
    }

    #[test]
    fn test_item_count_changed() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let mut after = before.clone();
        after.player_bag.as_mut().unwrap().items[0].item_count = 10;

        let delta = diff_player_data(&before, &after).unwrap();
        let bag = delta.bag.unwrap();
        assert_eq!(bag.item_changes.len(), 1);
        assert_eq!(bag.item_changes[0].op, DeltaOp::Upsert as i32);
        assert_eq!(bag.item_changes[0].item_count, 10);
    }

    #[test]
    fn test_multiple_changes() {
        let before = make_player_data(1, 100, 50, vec![(1001, 5)]);
        let mut after = before.clone();
        after.player_base.as_mut().unwrap().player_level = 2;
        after.player_bag.as_mut().unwrap().money[0].money_count = 150;
        after.player_bag.as_mut().unwrap().items[0].item_count = 8;
        after.player_bag.as_mut().unwrap().items.push(PlayerBagItem { item_id: 2001, item_count: 1 });

        let delta = diff_player_data(&before, &after).unwrap();
        assert_eq!(delta.base.unwrap().player_level, Some(2));
        let bag = delta.bag.unwrap();
        assert_eq!(bag.money_changes.len(), 1);
        assert_eq!(bag.item_changes.len(), 2); // update 1001 + add 2001
    }
}
