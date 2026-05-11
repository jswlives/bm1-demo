use std::collections::HashMap; // 哈希表，按 ID 存储会话
use std::time::Instant; // 单调时钟，记录会话最后活跃时间

/// 单个客户端会话
pub struct Session {
    pub id: u32,            // 会话唯一标识
    pub connected: bool,    // 是否在线（false 表示已断开，可重连）
    pub last_active: Instant, // 最后活跃时间戳
}

/// 会话管理器：统一管理所有客户端会话的创建、重连、断开
pub struct SessionManager {
    sessions: HashMap<u32, Session>, // 会话 ID → Session 的映射
    next_id: u32,                    // 下一个可分配的会话 ID
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(), // 空映射
            next_id: 1,              // 会话 ID 从 1 开始分配
        }
    }

    /// 创建新会话，返回分配的会话 ID
    pub fn create_session(&mut self) -> u32 {
        let id = self.next_id;   // 取当前 ID
        self.next_id += 1;       // 自增，为下次分配准备
        self.sessions.insert(    // 插入新会话记录
            id,
            Session {
                id,                                  // 会话 ID
                connected: true,                     // 标记为在线
                last_active: Instant::now(),         // 记录创建时间
            },
        );
        id // 返回分配的 ID
    }

    /// 尝试重连已有会话。成功返回 true，失败（不存在或已在线）返回 false
    pub fn reconnect(&mut self, id: u32) -> bool {
        if let Some(session) = self.sessions.get_mut(&id) { // 查找会话
            if !session.connected { // 只有已断开的会话才能重连
                session.connected = true;             // 标记为在线
                session.last_active = Instant::now(); // 刷新活跃时间
                return true; // 重连成功
            }
        }
        false // 会话不存在或已在线，重连失败
    }

    /// 断开会话（标记为离线，但不删除记录，以支持后续重连）
    pub fn disconnect(&mut self, id: u32) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.connected = false; // 标记为离线
        }
    }

    /// 根据会话 ID 查找会话（只读引用）
    pub fn get_session(&self, id: u32) -> Option<&Session> {
        self.sessions.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session(); // 创建第一个会话
        assert_eq!(id, 1);             // 第一个 ID 应为 1
        let session = mgr.get_session(id).unwrap();
        assert!(session.connected);    // 新会话应在线
    }

    #[test]
    fn test_disconnect_and_reconnect() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session(); // 创建会话

        mgr.disconnect(id); // 断开
        assert!(!mgr.get_session(id).unwrap().connected); // 应离线

        let result = mgr.reconnect(id); // 重连
        assert!(result);                // 应成功
        assert!(mgr.get_session(id).unwrap().connected); // 应在线
    }

    #[test]
    fn test_reconnect_nonexistent_session() {
        let mut mgr = SessionManager::new();
        assert!(!mgr.reconnect(999)); // 不存在的会话，重连应失败
    }

    #[test]
    fn test_reconnect_already_connected() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session(); // 创建会话（已在线）
        assert!(!mgr.reconnect(id));   // 在线会话重连应失败
    }
}
