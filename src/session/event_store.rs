use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::event_types::{SessionEvent, SessionHeader};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("session not found: {0}")]
    NotFound(Uuid),
    #[error("version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    #[error("duplicate sequence: {0}")]
    DuplicateSequence(u64),
}

#[async_trait]
pub trait SessionEventStore: Send + Sync {
    async fn create_session(&self, header: &SessionHeader) -> Result<(), SessionError>;
    async fn get_header(&self, session_id: Uuid) -> Result<Option<SessionHeader>, SessionError>;

    async fn append_event(&self, event: &SessionEvent) -> Result<(), SessionError>;
    async fn append_events(&self, events: &[SessionEvent]) -> Result<(), SessionError>;

    async fn get_events(
        &self,
        session_id: Uuid,
        from_sequence: u64,
        limit: Option<usize>,
    ) -> Result<Vec<SessionEvent>, SessionError>;

    async fn get_latest_sequence(&self, session_id: Uuid) -> Result<u64, SessionError>;

    async fn list_sessions(&self) -> Result<Vec<Uuid>, SessionError>;

    async fn delete_session(&self, session_id: Uuid) -> Result<(), SessionError>;
}

pub struct InMemoryEventStore {
    sessions: dashmap::DashMap<Uuid, SessionHeader>,
    events: dashmap::DashMap<Uuid, Vec<SessionEvent>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            sessions: dashmap::DashMap::new(),
            events: dashmap::DashMap::new(),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionEventStore for InMemoryEventStore {
    async fn create_session(&self, header: &SessionHeader) -> Result<(), SessionError> {
        self.sessions.insert(header.id, header.clone());
        self.events.insert(header.id, Vec::new());
        Ok(())
    }

    async fn get_header(&self, session_id: Uuid) -> Result<Option<SessionHeader>, SessionError> {
        Ok(self.sessions.get(&session_id).map(|h| h.clone()))
    }

    async fn append_event(&self, event: &SessionEvent) -> Result<(), SessionError> {
        let mut events = self.events.entry(event.session_id).or_insert_with(Vec::new);

        if let Some(last) = events.last() {
            if event.sequence <= last.sequence {
                return Err(SessionError::DuplicateSequence(event.sequence));
            }
        }

        events.push(event.clone());
        Ok(())
    }

    async fn append_events(&self, events: &[SessionEvent]) -> Result<(), SessionError> {
        for event in events {
            self.append_event(event).await?;
        }
        Ok(())
    }

    async fn get_events(
        &self,
        session_id: Uuid,
        from_sequence: u64,
        limit: Option<usize>,
    ) -> Result<Vec<SessionEvent>, SessionError> {
        let events = self.events.get(&session_id)
            .map(|e| e.clone())
            .unwrap_or_default();

        let filtered: Vec<SessionEvent> = events
            .into_iter()
            .filter(|e| e.sequence >= from_sequence)
            .collect();

        Ok(match limit {
            Some(n) => filtered.into_iter().take(n).collect(),
            None => filtered,
        })
    }

    async fn get_latest_sequence(&self, session_id: Uuid) -> Result<u64, SessionError> {
        let events = self.events.get(&session_id)
            .map(|e| e.clone())
            .unwrap_or_default();

        Ok(events.last().map(|e| e.sequence).unwrap_or(0))
    }

    async fn list_sessions(&self) -> Result<Vec<Uuid>, SessionError> {
        Ok(self.sessions.iter().map(|entry| *entry.key()).collect())
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<(), SessionError> {
        self.sessions.remove(&session_id);
        self.events.remove(&session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store_create_and_get() {
        let store = InMemoryEventStore::new();
        let header = SessionHeader::new(Uuid::new_v4());

        store.create_session(&header).await.unwrap();

        let loaded = store.get_header(header.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, header.id);
    }

    #[tokio::test]
    async fn test_in_memory_store_append_events() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);

        store.create_session(&header).await.unwrap();

        let event1 = SessionEvent::user_message(session_id, "Hello", 1);
        let event2 = SessionEvent::assistant_message(session_id, "Hi there", 2);

        store.append_event(&event1).await.unwrap();
        store.append_event(&event2).await.unwrap();

        let events = store.get_events(session_id, 0, None).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_get_events_from_sequence() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);

        store.create_session(&header).await.unwrap();

        for i in 1..=5 {
            let event = SessionEvent::user_message(session_id, &format!("msg {i}"), i);
            store.append_event(&event).await.unwrap();
        }

        let events = store.get_events(session_id, 3, None).await.unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].sequence, 3);
    }

    #[tokio::test]
    async fn test_in_memory_store_get_events_with_limit() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);

        store.create_session(&header).await.unwrap();

        for i in 1..=5 {
            let event = SessionEvent::user_message(session_id, &format!("msg {i}"), i);
            store.append_event(&event).await.unwrap();
        }

        let events = store.get_events(session_id, 0, Some(2)).await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_in_memory_store_duplicate_sequence() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);

        store.create_session(&header).await.unwrap();

        let event1 = SessionEvent::user_message(session_id, "Hello", 1);
        store.append_event(&event1).await.unwrap();

        let event2 = SessionEvent::user_message(session_id, "Hello again", 1);
        let result = store.append_event(&event2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_in_memory_store_latest_sequence() {
        let store = InMemoryEventStore::new();
        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);

        store.create_session(&header).await.unwrap();

        assert_eq!(store.get_latest_sequence(session_id).await.unwrap(), 0);

        let event = SessionEvent::user_message(session_id, "Hello", 5);
        store.append_event(&event).await.unwrap();

        assert_eq!(store.get_latest_sequence(session_id).await.unwrap(), 5);
    }
}
