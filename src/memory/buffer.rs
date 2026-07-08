use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryError, MemoryResult};
use crate::schema::Message;

/// 对话缓冲区 Memory
///
/// 简单的列表存储所有消息，无大小限制。
/// 适合短对话场景。
pub struct ConversationBufferMemory {
    store: Arc<RwLock<DashMap<String, Vec<Message>>>>,
    return_messages: bool,
}

impl ConversationBufferMemory {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(DashMap::new())),
            return_messages: true,
        }
    }

    pub fn with_return_messages(mut self, return_messages: bool) -> Self {
        self.return_messages = return_messages;
        self
    }
}

impl Default for ConversationBufferMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Memory for ConversationBufferMemory {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>> {
        let store = self.store.read().await;
        Ok(store
            .get(key)
            .map(|entry| entry.value().clone())
            .unwrap_or_default())
    }

async fn save(&self, key: &str, message: Message) -> MemoryResult<()> {
        let mut store = self.store.write().await;
        if !store.contains_key(key) {
            store.insert(key.to_string(), Vec::new());
        }
        let mut entry = store.get_mut(key).unwrap();
        entry.push(message);
        Ok(())
    }

    async fn save_batch(&self, key: &str, messages: Vec<Message>) -> MemoryResult<()> {
        let mut store = self.store.write().await;
        if !store.contains_key(key) {
            store.insert(key.to_string(), Vec::new());
        }
        let mut entry = store.get_mut(key).unwrap();
        entry.extend(messages);
        Ok(())
    }

    async fn clear(&self, key: &str) -> MemoryResult<()> {
        let mut store = self.store.write().await;
        store.remove(key);
        Ok(())
    }

    async fn size(&self, key: &str) -> MemoryResult<usize> {
        let store = self.store.read().await;
        Ok(store.get(key).map(|e| e.value().len()).unwrap_or(0))
    }
}

impl ConversationBufferMemory {
    pub fn return_messages(&self) -> bool {
        self.return_messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Content, RoleType};

    fn user_msg(text: &str) -> Message {
        Message {
            role: RoleType::User,
            content: Content::Text(text.to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    #[tokio::test]
    async fn test_buffer_memory_basic() {
        let memory = ConversationBufferMemory::new();
        memory.save("k1", user_msg("hello")).await.unwrap();
        memory.save("k1", user_msg("world")).await.unwrap();

        let msgs = memory.load("k1").await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn test_buffer_memory_isolation() {
        let memory = ConversationBufferMemory::new();
        memory.save("k1", user_msg("a")).await.unwrap();
        memory.save("k2", user_msg("b")).await.unwrap();

        assert_eq!(memory.load("k1").await.unwrap().len(), 1);
        assert_eq!(memory.load("k2").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_buffer_memory_clear() {
        let memory = ConversationBufferMemory::new();
        memory.save("k1", user_msg("a")).await.unwrap();
        memory.clear("k1").await.unwrap();
        assert!(memory.load("k1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_buffer_memory_empty_load() {
        let memory = ConversationBufferMemory::new();
        let msgs = memory.load("nonexistent").await.unwrap();
        assert!(msgs.is_empty());
    }
}