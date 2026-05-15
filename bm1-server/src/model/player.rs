use bm1_proto::model::{PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerBagMoneyType, PlayerBase, PlayerData, PlayerEquip, PlayerEquipData, PlayerSkill, PlayerSkillData};

/// 加载后的玩家实例，封装对 PlayerData 的数据操作
pub struct Player {
    data: PlayerData,
}

impl Player {
    pub fn new(data: PlayerData) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &PlayerData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut PlayerData {
        &mut self.data
    }

    // ---- PlayerBase ----

    pub fn player_id(&self) -> u64 {
        self.data.player_base.as_ref().map(|b| b.player_id).unwrap_or(0)
    }

    pub fn player_name(&self) -> &str {
        self.data.player_base.as_ref().map(|b| b.player_name.as_str()).unwrap_or("")
    }

    pub fn set_player_name(&mut self, name: String) {
        self.ensure_base().player_name = name;
    }

    pub fn level(&self) -> u32 {
        self.data.player_base.as_ref().map(|b| b.player_level).unwrap_or(0)
    }

    pub fn set_level(&mut self, level: u32) {
        self.ensure_base().player_level = level;
    }

    pub fn add_level(&mut self, delta: u32) -> u32 {
        let base = self.ensure_base();
        base.player_level = base.player_level.saturating_add(delta);
        base.player_level
    }

    // ---- Money ----

    fn ensure_bag(&mut self) -> &mut PlayerBag {
        if self.data.player_bag.is_none() {
            self.data.player_bag = Some(PlayerBag::default());
        }
        self.data.player_bag.as_mut().unwrap()
    }

    fn ensure_base(&mut self) -> &mut PlayerBase {
        if self.data.player_base.is_none() {
            self.data.player_base = Some(PlayerBase::default());
        }
        self.data.player_base.as_mut().unwrap()
    }

    fn find_money(&self, money_type: PlayerBagMoneyType) -> Option<&PlayerBagMoney> {
        let ty = money_type as i32;
        self.data.player_bag.as_ref()?.money.iter().find(|m| m.money_type == ty)
    }

    fn find_money_mut(&mut self, money_type: PlayerBagMoneyType) -> Option<&mut PlayerBagMoney> {
        let ty = money_type as i32;
        let bag = self.ensure_bag();
        bag.money.iter_mut().find(|m| m.money_type == ty)
    }

    pub fn get_money(&self, money_type: PlayerBagMoneyType) -> u32 {
        self.find_money(money_type).map(|m| m.money_count).unwrap_or(0)
    }

    pub fn add_money(&mut self, money_type: PlayerBagMoneyType, amount: u32) -> u32 {
        if let Some(money) = self.find_money_mut(money_type) {
            money.money_count = money.money_count.saturating_add(amount);
            money.money_count
        } else {
            self.ensure_bag().money.push(PlayerBagMoney {
                money_type: money_type as i32,
                money_count: amount,
            });
            amount
        }
    }

    pub fn sub_money(&mut self, money_type: PlayerBagMoneyType, amount: u32) -> Result<u32, &'static str> {
        let money = self.find_money_mut(money_type).ok_or("money type not found")?;
        if money.money_count < amount {
            return Err("insufficient money");
        }
        money.money_count -= amount;
        Ok(money.money_count)
    }

    // ---- Items ----

    fn find_item(&self, item_id: u32) -> Option<&PlayerBagItem> {
        self.data.player_bag.as_ref()?.items.iter().find(|i| i.item_id == item_id)
    }

    fn find_item_mut(&mut self, item_id: u32) -> Option<&mut PlayerBagItem> {
        let bag = self.ensure_bag();
        bag.items.iter_mut().find(|i| i.item_id == item_id)
    }

    pub fn get_item_count(&self, item_id: u32) -> u32 {
        self.find_item(item_id).map(|i| i.item_count).unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: u32, count: u32) -> u32 {
        if let Some(item) = self.find_item_mut(item_id) {
            item.item_count = item.item_count.saturating_add(count);
            item.item_count
        } else {
            self.ensure_bag().items.push(PlayerBagItem { item_id, item_count: count });
            count
        }
    }

    pub fn sub_item(&mut self, item_id: u32, count: u32) -> Result<u32, &'static str> {
        let item = self.find_item_mut(item_id).ok_or("item not found")?;
        if item.item_count < count {
            return Err("insufficient item count");
        }
        item.item_count -= count;
        let remaining = item.item_count;
        // 数量归零时移除道具
        if remaining == 0 {
            self.ensure_bag().items.retain(|i| i.item_id != item_id);
        }
        Ok(remaining)
    }

    // ---- Convenience ----

    pub fn gold(&self) -> u32 {
        self.get_money(PlayerBagMoneyType::Gold)
    }

    pub fn add_gold(&mut self, amount: u32) -> u32 {
        self.add_money(PlayerBagMoneyType::Gold, amount)
    }

    pub fn sub_gold(&mut self, amount: u32) -> Result<u32, &'static str> {
        self.sub_money(PlayerBagMoneyType::Gold, amount)
    }

    pub fn diamond(&self) -> u32 {
        self.get_money(PlayerBagMoneyType::Diamond)
    }

    pub fn add_diamond(&mut self, amount: u32) -> u32 {
        self.add_money(PlayerBagMoneyType::Diamond, amount)
    }

    pub fn sub_diamond(&mut self, amount: u32) -> Result<u32, &'static str> {
        self.sub_money(PlayerBagMoneyType::Diamond, amount)
    }

    pub fn add_exp(&mut self, exp: u32) -> u32 {
        // 简单实现：经验直接加到等级上，后续可替换为经验表
        self.add_level(exp)
    }

    // ---- Skill ----

    fn ensure_skill(&mut self) -> &mut PlayerSkillData {
        if self.data.player_skill.is_none() {
            self.data.player_skill = Some(PlayerSkillData::default());
        }
        self.data.player_skill.as_mut().unwrap()
    }

    pub fn skill_points(&self) -> u32 {
        self.data.player_skill.as_ref().map(|s| s.skill_points).unwrap_or(0)
    }

    pub fn add_skill_points(&mut self, amount: u32) -> u32 {
        self.ensure_skill().skill_points = self.ensure_skill().skill_points.saturating_add(amount);
        self.skill_points()
    }

    pub fn skill_level(&self, skill_id: u32) -> Option<u32> {
        self.data.player_skill.as_ref()?
            .skills.iter()
            .find(|s| s.skill_id == skill_id)
            .map(|s| s.skill_level)
    }

    pub fn unlock_skill(&mut self, skill_id: u32) -> Result<(u32, u32), &'static str> {
        if self.skill_level(skill_id).is_some() {
            return Err("skill already unlocked");
        }
        let skill = self.ensure_skill();
        if skill.skill_points < 1 {
            return Err("insufficient skill points");
        }
        skill.skill_points -= 1;
        skill.skills.push(PlayerSkill { skill_id, skill_level: 1 });
        Ok((skill_id, 1))
    }

    pub fn upgrade_skill(&mut self, skill_id: u32) -> Result<(u32, u32), &'static str> {
        let skill = self.ensure_skill();
        let existing = skill.skills.iter_mut().find(|s| s.skill_id == skill_id)
            .ok_or("skill not unlocked")?;
        if skill.skill_points < 1 {
            return Err("insufficient skill points");
        }
        skill.skill_points -= 1;
        existing.skill_level = existing.skill_level.saturating_add(1);
        Ok((skill_id, existing.skill_level))
    }

    // ---- Equip ----

    fn ensure_equip(&mut self) -> &mut PlayerEquipData {
        if self.data.player_equip.is_none() {
            self.data.player_equip = Some(PlayerEquipData::default());
        }
        self.data.player_equip.as_mut().unwrap()
    }

    fn find_equip(&self, equip_id: u32) -> Option<&PlayerEquip> {
        self.data.player_equip.as_ref()?
            .equips.iter()
            .find(|e| e.equip_id == equip_id)
    }

    pub fn equip_level(&self, equip_id: u32) -> Option<u32> {
        self.find_equip(equip_id).map(|e| e.equip_level)
    }

    pub fn buy_equip(&mut self, equip_id: u32) -> Result<(u32, u32), &'static str> {
        if self.find_equip(equip_id).is_some() {
            return Err("equip already owned");
        }
        self.sub_gold(500)?;
        self.ensure_equip().equips.push(PlayerEquip { equip_id, equip_level: 1 });
        Ok((equip_id, 1))
    }

    pub fn upgrade_equip(&mut self, equip_id: u32) -> Result<(u32, u32), &'static str> {
        // validate ownership first
        if self.find_equip(equip_id).is_none() {
            return Err("equip not owned");
        }
        self.sub_gold(100)?;
        let equip = self.ensure_equip().equips.iter_mut()
            .find(|e| e.equip_id == equip_id)
            .unwrap(); // safe: validated above
        equip.equip_level = equip.equip_level.saturating_add(1);
        Ok((equip_id, equip.equip_level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> Player {
        Player::new(PlayerData {
            player_base: Some(PlayerBase {
                player_id: 1001,
                player_name: "test".into(),
                player_level: 1,
            }),
            player_bag: Some(PlayerBag {
                items: vec![PlayerBagItem { item_id: 100, item_count: 5 }],
                money: vec![PlayerBagMoney { money_type: PlayerBagMoneyType::Gold as i32, money_count: 100 }],
            }),
            player_skill: None,
            player_equip: None,
        })
    }

    #[test]
    fn test_base_access() {
        let p = test_player();
        assert_eq!(p.player_id(), 1001);
        assert_eq!(p.player_name(), "test");
        assert_eq!(p.level(), 1);
    }

    #[test]
    fn test_add_level() {
        let mut p = test_player();
        assert_eq!(p.add_level(2), 3);
    }

    #[test]
    fn test_money_operations() {
        let mut p = test_player();
        assert_eq!(p.gold(), 100);
        assert_eq!(p.add_gold(50), 150);
        assert_eq!(p.sub_gold(30).unwrap(), 120);
        assert!(p.sub_gold(200).is_err());
        assert_eq!(p.diamond(), 0);
        assert_eq!(p.add_diamond(10), 10);
    }

    #[test]
    fn test_item_operations() {
        let mut p = test_player();
        assert_eq!(p.get_item_count(100), 5);
        assert_eq!(p.add_item(100, 3), 8);
        assert_eq!(p.sub_item(100, 8).unwrap(), 0);
        assert_eq!(p.get_item_count(100), 0); // 归零后移除
        assert_eq!(p.add_item(200, 1), 1);
        assert!(p.sub_item(999, 1).is_err());
        assert!(p.sub_item(200, 5).is_err());
    }

    #[test]
    fn test_empty_player() {
        let mut p = Player::new(PlayerData::default());
        assert_eq!(p.player_id(), 0);
        assert_eq!(p.gold(), 0);
        assert_eq!(p.add_gold(10), 10);
        assert_eq!(p.add_item(1, 1), 1);
    }

    #[test]
    fn test_buy_equip_success() {
        let mut p = test_player();
        p.add_gold(500); // 100 + 500 = 600
        let (id, level) = p.buy_equip(1001).unwrap();
        assert_eq!(id, 1001);
        assert_eq!(level, 1);
        assert_eq!(p.gold(), 100);
        assert_eq!(p.equip_level(1001), Some(1));
    }

    #[test]
    fn test_buy_equip_already_owned() {
        let mut p = test_player();
        p.add_gold(500);
        p.buy_equip(1001).unwrap();
        p.add_gold(500);
        assert!(p.buy_equip(1001).is_err());
    }

    #[test]
    fn test_buy_equip_insufficient_gold() {
        let mut p = test_player();
        assert!(p.buy_equip(1001).is_err()); // 100 < 500
    }

    #[test]
    fn test_upgrade_equip_success() {
        let mut p = test_player();
        p.add_gold(600); // 100 + 600 = 700, enough for buy(500) + upgrade(100)
        p.buy_equip(1001).unwrap();
        let (id, level) = p.upgrade_equip(1001).unwrap();
        assert_eq!(id, 1001);
        assert_eq!(level, 2);
        assert_eq!(p.gold(), 100);
    }

    #[test]
    fn test_upgrade_equip_not_owned() {
        let mut p = test_player();
        assert!(p.upgrade_equip(999).is_err());
    }

    #[test]
    fn test_upgrade_equip_insufficient_gold() {
        let mut p = test_player();
        p.add_gold(500); // 600 total, buy costs 500, leaves 100
        p.buy_equip(1001).unwrap();
        // gold = 100, upgrade costs 100 exactly, but then next upgrade fails
        assert!(p.upgrade_equip(1001).is_ok()); // 100 - 100 = 0
        p.add_gold(50); // 50 < 100
        assert!(p.upgrade_equip(1001).is_err());
    }
}
