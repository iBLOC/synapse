use crate::schema::Message;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

pub struct Session {
    pub id: Uuid,
    pub name: Option<String>,
    pub messages: RwLock<Vec<Message>>,
    pub metadata: DashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: RwLock<chrono::DateTime<chrono::Utc>>,
    remaining_iterations: RwLock<usize>,
}

impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        let id = Uuid::new_v4();
        Self {
            id,
            name: None,
            messages: RwLock::new(Vec::new()),
            metadata: DashMap::new(),
            created_at: now,
            updated_at: RwLock::new(now),
            remaining_iterations: RwLock::new(100),
        }
    }

    pub fn with_id(id: Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name: None,
            messages: RwLock::new(Vec::new()),
            metadata: DashMap::new(),
            created_at: now,
            updated_at: RwLock::new(now),
            remaining_iterations: RwLock::new(100),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_max_iterations(self, max: usize) -> Self {
        *self.remaining_iterations.write() = max;
        self
    }

    pub fn add_message(&self, msg: Message) {
        self.messages.write().push(msg);
        *self.updated_at.write() = chrono::Utc::now();
    }

    pub fn get_messages(&self) -> Vec<Message> {
        self.messages.read().clone()
    }

    pub fn decrement_iterations(&self) -> bool {
        let mut remaining = self.remaining_iterations.write();
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        true
    }

    pub fn remaining_iterations(&self) -> usize {
        *self.remaining_iterations.read()
    }

    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.get(key).map(|v| v.to_string())
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl serde::Serialize for Session {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Session", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("messages", &*self.messages.read())?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &*self.updated_at.read())?;
        state.end()
    }
}

pub struct SessionManager {
    sessions: DashMap<Uuid, Arc<Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn create_session(&self) -> Arc<Session> {
        let session = Arc::new(Session::new());
        self.sessions.insert(session.id, session.clone());
        session
    }

    pub fn create_session_with(&self, name: impl Into<String>) -> Arc<Session> {
        let session = Arc::new(Session::new().with_name(name));
        self.sessions.insert(session.id, session.clone());
        session
    }

    pub fn get_session(&self, id: Uuid) -> Option<Arc<Session>> {
        self.sessions.get(&id).map(|s| s.value().clone())
    }

    pub fn remove_session(&self, id: Uuid) -> Option<Arc<Session>> {
        self.sessions.remove(&id).map(|s| s.1)
    }

    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.iter().map(|s| *s.key()).collect()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
