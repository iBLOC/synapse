use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryResult};
use crate::schema::Message;

/// 滑动窗口 Memory
///
/// 只保留最近 N 条消息，旧消息自动丢弃。
/// 适合长对话场景，避免 token 过多。
pub struct WindowMemory {
    store: Arc<RwLock<DashMap<String, Vec<Message>>>>,
    window_size: usize,
}

impl WindowMemory {
    pub fn new(window_size: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(DashMap::new())),
            window_size: window_size.max(1),
        }
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

impl Default for WindowMemory {
    fn default() -> Self {
        Self::new(10)
    }
}

#[async_trait]
impl Memory for WindowMemory {
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
        let len = entry.len();
        if len > self.window_size {
            let drop_count = len - self.window_size;
            entry.drain(0..drop_count);
        }
        Ok(())
    }

    async fn save_batch(&self, key: &str, messages: Vec<Message>) -> MemoryResult<()> {
        let mut store = self.store.write().await;
        if !store.contains_key(key) {
            store.insert(key.to_string(), Vec::new());
        }
        let mut entry = store.get_mut(key).unwrap();
        entry.extend(messages);
        let len = entry.len();
        if len > self.window_size {
            let drop_count = len - self.window_size;
            entry.drain(0..drop_count);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Content, RoleType};

    fn msg(text: &str) -> Message {
        Message {
            role: RoleType::User,
            content: Content::Text(text.to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    #[tokio::test]
    async fn test_window_memory_basic() {
        let memory = WindowMemory::new(3);
        memory.save("k1", msg("1")).await.unwrap();
        memory.save("k1", msg("2")).await.unwrap();
        memory.save("k1", msg("3")).await.unwrap();
        memory.save("k1", msg("4")).await.unwrap();

        let msgs = memory.load("k1").await.unwrap();
        assert_eq!(msgs.len(), 3);
        let text0 = match &msgs[0].content {
            crate::schema::Content::Text(t) => t.clone(),
            _ => String::new(),
        };
        let text2 = match &msgs[2].content {
            crate::schema::Content::Text(t) => t.clone(),
            _ => String::new(),
        };
        assert_eq!(text0, "2");
        assert_eq!(text2, "4");
    }

    #[tokio::test]
    async fn test_window_memory_clear() {
        let memory = WindowMemory::new(5);
        memory.save("k1", msg("a")).await.unwrap();
        memory.clear("k1").await.unwrap();
        assert!(memory.load("k1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_window_memory_default() {
        let memory = WindowMemory::default();
        assert_eq!(memory.window_size(), 10);
    }
}