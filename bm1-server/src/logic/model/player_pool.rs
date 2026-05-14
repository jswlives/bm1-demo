use std::collections::HashMap;

use super::player::Player;
use bm1_proto::model::PlayerData;

/// 已加载玩家实例的池，提供增删查操作
pub struct PlayerPool {
    players: HashMap<u64, Player>,
}

impl PlayerPool {
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
    use bm1_proto::model::{PlayerBase, PlayerBag};

    fn make_player(id: u64, name: &str) -> PlayerData {
        PlayerData {
            player_base: Some(PlayerBase {
                player_id: id,
                player_name: name.into(),
                player_level: 1,
            }),
            player_bag: Some(PlayerBag::default()),
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
    fn test_add_replace() {
        let mut pool = PlayerPool::new();
        pool.load(make_player(1, "alice"));

        let old = pool.load(make_player(1, "alice_v2"));
        assert_eq!(old.unwrap().player_name(), "alice");
        assert_eq!(pool.get(1).unwrap().player_name(), "alice_v2");
    }
}
