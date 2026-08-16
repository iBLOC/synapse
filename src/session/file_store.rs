use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::event_types::{SessionEvent, SessionHeader};
use super::event_store::{SessionError, SessionEventStore};

pub struct FileEventStore {
    base_dir: PathBuf,
}

impl FileEventStore {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, SessionError> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)
            .map_err(|e| SessionError::Io(format!("create dir: {e}")))?;
        Ok(Self { base_dir })
    }

    fn session_dir(&self, session_id: Uuid) -> PathBuf {
        self.base_dir.join(session_id.to_string())
    }

    fn header_path(&self, session_id: Uuid) -> PathBuf {
        self.session_dir(session_id).join("header.json")
    }

    fn events_path(&self, session_id: Uuid) -> PathBuf {
        self.session_dir(session_id).join("events.jsonl")
    }

    async fn read_events(&self, session_id: Uuid) -> Result<Vec<SessionEvent>, SessionError> {
        let path = self.events_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| SessionError::Io(format!("read events: {e}")))?;

        let mut events = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: SessionEvent = serde_json::from_str(line)
                .map_err(|e| SessionError::Serialization(format!("parse event: {e}")))?;
            events.push(event);
        }

        Ok(events)
    }
}

#[async_trait]
impl SessionEventStore for FileEventStore {
    async fn create_session(&self, header: &SessionHeader) -> Result<(), SessionError> {
        let dir = self.session_dir(header.id);
        tokio::fs::create_dir_all(&dir).await
            .map_err(|e| SessionError::Io(format!("create session dir: {e}")))?;

        let path = self.header_path(header.id);
        let json = serde_json::to_string_pretty(header)
            .map_err(|e| SessionError::Serialization(format!("serialize header: {e}")))?;
        tokio::fs::write(&path, json).await
            .map_err(|e| SessionError::Io(format!("write header: {e}")))?;

        let events_path = self.events_path(header.id);
        tokio::fs::write(&events_path, "").await
            .map_err(|e| SessionError::Io(format!("create events file: {e}")))?;

        Ok(())
    }

    async fn get_header(&self, session_id: Uuid) -> Result<Option<SessionHeader>, SessionError> {
        let path = self.header_path(session_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| SessionError::Io(format!("read header: {e}")))?;
        let header: SessionHeader = serde_json::from_str(&content)
            .map_err(|e| SessionError::Serialization(format!("parse header: {e}")))?;

        Ok(Some(header))
    }

    async fn append_event(&self, event: &SessionEvent) -> Result<(), SessionError> {
        let events = self.read_events(event.session_id).await?;

        if let Some(last) = events.last() {
            if event.sequence <= last.sequence {
                return Err(SessionError::DuplicateSequence(event.sequence));
            }
        }

        let path = self.events_path(event.session_id);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SessionError::Io(format!("open events file: {e}")))?;

        let json = serde_json::to_string(event)
            .map_err(|e| SessionError::Serialization(format!("serialize event: {e}")))?;
        file.write_all(json.as_bytes()).await
            .map_err(|e| SessionError::Io(format!("write event: {e}")))?;
        file.write_all(b"\n").await
            .map_err(|e| SessionError::Io(format!("write newline: {e}")))?;

        Ok(())
    }

    async fn append_events(&self, events: &[SessionEvent]) -> Result<(), SessionError> {
        if events.is_empty() {
            return Ok(());
        }

        let session_id = events[0].session_id;
        let existing = self.read_events(session_id).await?;

        if let Some(last) = existing.last() {
            if events[0].sequence <= last.sequence {
                return Err(SessionError::DuplicateSequence(events[0].sequence));
            }
        }

        let path = self.events_path(session_id);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| SessionError::Io(format!("open events file: {e}")))?;

        for event in events {
            let json = serde_json::to_string(event)
                .map_err(|e| SessionError::Serialization(format!("serialize event: {e}")))?;
            file.write_all(json.as_bytes()).await
                .map_err(|e| SessionError::Io(format!("write event: {e}")))?;
            file.write_all(b"\n").await
                .map_err(|e| SessionError::Io(format!("write newline: {e}")))?;
        }

        Ok(())
    }

    async fn get_events(
        &self,
        session_id: Uuid,
        from_sequence: u64,
        limit: Option<usize>,
    ) -> Result<Vec<SessionEvent>, SessionError> {
        let events = self.read_events(session_id).await?;

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
        let events = self.read_events(session_id).await?;
        Ok(events.last().map(|e| e.sequence).unwrap_or(0))
    }

    async fn list_sessions(&self) -> Result<Vec<Uuid>, SessionError> {
        let mut sessions = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_dir).await
            .map_err(|e| SessionError::Io(format!("read dir: {e}")))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| SessionError::Io(format!("read entry: {e}")))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Ok(id) = Uuid::parse_str(&name.to_string_lossy()) {
                        sessions.push(id);
                    }
                }
            }
        }

        Ok(sessions)
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<(), SessionError> {
        let dir = self.session_dir(session_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await
                .map_err(|e| SessionError::Io(format!("delete session: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_store_create_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileEventStore::new(dir.path()).unwrap();

        let header = SessionHeader::new(Uuid::new_v4());
        store.create_session(&header).await.unwrap();

        let loaded = store.get_header(header.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, header.id);
    }

    #[tokio::test]
    async fn test_file_store_append_and_get_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileEventStore::new(dir.path()).unwrap();

        let session_id = Uuid::new_v4();
        let header = SessionHeader::new(session_id);
        store.create_session(&header).await.unwrap();

        let event1 = SessionEvent::user_message(session_id, "Hello", 1);
        let event2 = SessionEvent::assistant_message(session_id, "Hi", 2);

        store.append_event(&event1).await.unwrap();
        store.append_event(&event2).await.unwrap();

        let events = store.get_events(session_id, 0, None).await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_file_store_survives_reload() {
        let dir = tempfile::tempdir().unwrap();
        let session_id = Uuid::new_v4();

        {
            let store = FileEventStore::new(dir.path()).unwrap();
            let header = SessionHeader::new(session_id);
            store.create_session(&header).await.unwrap();

            let event = SessionEvent::user_message(session_id, "persisted", 1);
            store.append_event(&event).await.unwrap();
        }

        let store = FileEventStore::new(dir.path()).unwrap();
        let events = store.get_events(session_id, 0, None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, 1);
    }

    #[tokio::test]
    async fn test_file_store_list_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileEventStore::new(dir.path()).unwrap();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        store.create_session(&SessionHeader::new(id1)).await.unwrap();
        store.create_session(&SessionHeader::new(id2)).await.unwrap();

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&id1));
        assert!(sessions.contains(&id2));
    }
}
