use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use bm1_proto::model::{PlayerBag, PlayerBagItem, PlayerBagMoney, PlayerBagMoneyType, PlayerBase, PlayerData};

use super::player::Player;

/// 已加载玩家实例的池，提供增删查操作
pub struct PlayerPool {
    players: HashMap<u64, Player>,
}

static PLAYER_POOL: LazyLock<RwLock<PlayerPool>> = LazyLock::new(|| {
    let mut pool = PlayerPool::new();
    pool.load(PlayerData {
        player_base: Some(PlayerBase {
            player_id: 1,
            player_name: "alice".into(),
            player_level: 10,
        }),
        player_bag: Some(PlayerBag {
            items: vec![PlayerBagItem { item_id: 1001, item_count: 5 }],
            money: vec![PlayerBagMoney {
                money_type: PlayerBagMoneyType::Gold as i32,
                money_count: 1000,
            }],
        }),
        player_skill: None,
        player_equip: None,
    });
    pool.load(PlayerData {
        player_base: Some(PlayerBase {
            player_id: 2,
            player_name: "bob".into(),
            player_level: 20,
        }),
        player_bag: Some(PlayerBag {
            items: vec![
                PlayerBagItem { item_id: 2001, item_count: 3 },
                PlayerBagItem { item_id: 2002, item_count: 1 },
            ],
            money: vec![
                PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Gold as i32,
                    money_count: 500,
                },
                PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Diamond as i32,
                    money_count: 50,
                },
            ],
        }),
        player_skill: None,
        player_equip: None,
    });
    RwLock::new(pool)
});

impl PlayerPool {
    /// 获取全局玩家池实例
    pub fn global() -> &'static RwLock<PlayerPool> {
        &PLAYER_POOL
    }

    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    /// 加载玩家到池中，若 player_id 已存在则替换并返回旧实例
    pub fn add(&mut self, player: Player) -> Option<Player> {
        self.players.insert(player.player_id(), player)
    }

    /// 从 PlayerData 创建并加载玩家，等价于 add(Player::new(data))
    pub fn load(&mut self, data: PlayerData) -> Option<Player> {
        let player = Player::new(data);
        self.add(player)
    }

    /// 按 player_id 卸载玩家，返回被移除的实例
    pub fn remove(&mut self, player_id: u64) -> Option<Player> {
        self.players.remove(&player_id)
    }

    /// 按 player_id 查找玩家（不可变引用）
    pub fn get(&self, player_id: u64) -> Option<&Player> {
        self.players.get(&player_id)
    }

    /// 按 player_id 查找玩家（可变引用）
    pub fn get_mut(&mut self, player_id: u64) -> Option<&mut Player> {
        self.players.get_mut(&player_id)
    }

    /// 当前池中玩家数量
    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    /// 玩家是否已在池中
    pub fn contains(&self, player_id: u64) -> bool {
        self.players.contains_key(&player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm1_proto::model::PlayerBase;

    fn make_player(id: u64, name: &str) -> PlayerData {
        PlayerData {
            player_base: Some(PlayerBase {
                player_id: id,
                player_name: name.into(),
                player_level: 1,
            }),
            player_bag: Some(PlayerBag::default()),
            player_skill: None,
            player_equip: None,
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut pool = PlayerPool::new();
        assert!(pool.is_empty());

        pool.load(make_player(1, "alice"));
        pool.load(make_player(2, "bob"));

        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(1).unwrap().player_name(), "alice");
        assert!(pool.contains(2));
        assert!(!pool.contains(3));
    }

    #[test]
    fn test_remove() {
        let mut pool = PlayerPool::new();
        pool.load(make_player(1, "alice"));

        let removed = pool.remove(1).unwrap();
        assert_eq!(removed.player_name(), "alice");
        assert!(pool.is_empty());
        assert!(pool.remove(999).is_none());
    }

    #[test]
    fn test_get_mut() {
        let mut pool = PlayerPool::new();
        pool.load(make_player(1, "alice"));

        pool.get_mut(1).unwrap().add_gold(100);
        assert_eq!(pool.get(1).unwrap().gold(), 100);
    }

    #[test]
    fn test_global_mut() {
        let mut pool = PlayerPool::global().write().unwrap();
        pool.load(PlayerData {
            player_base: Some(PlayerBase {
                player_id: 104,
                player_name: "alice".into(),
                player_level: 10,
            }),
            player_bag: Some(PlayerBag {
                items: vec![],
                money: vec![PlayerBagMoney {
                    money_type: PlayerBagMoneyType::Gold as i32,
                    money_count: 1000,
                }],
            }),
            player_skill: None,
            player_equip: None,
        });
        pool.get_mut(104).unwrap().add_gold(100);
        assert_eq!(pool.get(104).unwrap().gold(), 1100);
        pool.get_mut(104).unwrap().sub_gold(100).unwrap();
    }

    #[test]
    fn test_add_replace() {
        let mut pool = PlayerPool::new();
        pool.load(make_player(1, "alice"));

        let old = pool.load(make_player(1, "alice_v2"));
        assert_eq!(old.unwrap().player_name(), "alice");
        assert_eq!(pool.get(1).unwrap().player_name(), "alice_v2");
    }
}
