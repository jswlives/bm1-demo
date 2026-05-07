use std::collections::HashMap;
use std::time::Instant;

pub struct Session {
    pub id: u32,
    pub connected: bool,
    pub last_active: Instant,
}

pub struct SessionManager {
    sessions: HashMap<u32, Session>,
    next_id: u32,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_session(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.sessions.insert(
            id,
            Session {
                id,
                connected: true,
                last_active: Instant::now(),
            },
        );
        id
    }

    pub fn reconnect(&mut self, id: u32) -> bool {
        if let Some(session) = self.sessions.get_mut(&id) {
            if !session.connected {
                session.connected = true;
                session.last_active = Instant::now();
                return true;
            }
        }
        false
    }

    pub fn disconnect(&mut self, id: u32) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.connected = false;
        }
    }

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
        let id = mgr.create_session();
        assert_eq!(id, 1);
        let session = mgr.get_session(id).unwrap();
        assert!(session.connected);
    }

    #[test]
    fn test_disconnect_and_reconnect() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session();

        mgr.disconnect(id);
        assert!(!mgr.get_session(id).unwrap().connected);

        let result = mgr.reconnect(id);
        assert!(result);
        assert!(mgr.get_session(id).unwrap().connected);
    }

    #[test]
    fn test_reconnect_nonexistent_session() {
        let mut mgr = SessionManager::new();
        assert!(!mgr.reconnect(999));
    }

    #[test]
    fn test_reconnect_already_connected() {
        let mut mgr = SessionManager::new();
        let id = mgr.create_session();
        assert!(!mgr.reconnect(id));
    }
}
