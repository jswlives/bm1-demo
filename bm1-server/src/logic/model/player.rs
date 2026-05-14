use bm1_proto::model::{PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerBagMoneyType, PlayerBase, PlayerData};

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
}
