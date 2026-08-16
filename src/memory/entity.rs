use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryResult};
use crate::schema::Message;

/// 实体 Memory
///
/// 从对话中抽取和跟踪实体（如人名、地点、事件）。
/// 使用关键词匹配进行简单的实体提取。
pub struct EntityMemory {
    store: Arc<RwLock<DashMap<String, EntityStore>>>,
    keywords: Vec<String>,
}

struct EntityStore {
    entities: DashMap<String, Vec<String>>,
}

impl EntityMemory {
    pub fn new() -> Self {
        let keywords = vec![
            "name", "我叫", "我是", "住", "在",
            "is", "lives", "from", "at", "in",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            store: Arc::new(RwLock::new(DashMap::new())),
            keywords,
        }
    }

    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    fn extract_entities(&self, text: &str) -> Vec<(String, String)> {
        let mut entities = Vec::new();
        let lower = text.to_lowercase();
        for kw in &self.keywords {
            if let Some(idx) = lower.find(&kw.to_lowercase()) {
                let after = &text[idx + kw.len()..];
                let value: String = after
                    .chars()
                    .take_while(|c| !c.is_ascii_punctuation() || *c == ' ' || *c == ',')
                    .take(50)
                    .collect();
                let value = value.trim().to_string();
                if !value.is_empty() {
                    entities.push((kw.clone(), value));
                }
            }
        }
        entities
    }
}

impl Default for EntityMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Memory for EntityMemory {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>> {
        let store = self.store.read().await;
        let Some(state) = store.get(key) else {
            return Ok(Vec::new());
        };

        let mut entities: Vec<(String, Vec<String>)> = state
            .entities
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        entities.sort_by(|a, b| a.0.cmp(&b.0));

        let mut summary = String::from("Known entities:\n");
        for (kind, values) in entities {
            for v in values {
                summary.push_str(&format!("- {}: {}\n", kind, v));
            }
        }

        Ok(vec![Message {
            role: crate::schema::RoleType::System,
            content: crate::schema::Content::Text(summary),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }])
    }

    async fn save(&self, key: &str, message: Message) -> MemoryResult<()> {
        let text = match &message.content {
            crate::schema::Content::Text(t) => t.clone(),
            _ => return Ok(()),
        };

        let entities = self.extract_entities(&text);
        if entities.is_empty() {
            return Ok(());
        }

        let mut store = self.store.write().await;
        let state = store.entry(key.to_string()).or_insert(EntityStore {
            entities: DashMap::new(),
        });

        for (kind, value) in entities {
            state
                .entities
                .entry(kind)
                .or_default()
                .push(value);
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
        Ok(store
            .get(key)
            .map(|s| s.entities.iter().map(|e| e.value().len()).sum())
            .unwrap_or(0))
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
    async fn test_entity_memory_basic() {
        let memory = EntityMemory::new();
        memory.save("k1", msg("My name is Alice")).await.unwrap();
        memory.save("k1", msg("I live in Beijing")).await.unwrap();

        let msgs = memory.load("k1").await.unwrap();
        assert_eq!(msgs.len(), 1);

        let text = match &msgs[0].content {
            Content::Text(t) => t.clone(),
            _ => String::new(),
        };
        assert!(text.contains("name") || text.contains("Alice"));
    }

    #[tokio::test]
    async fn test_entity_memory_clear() {
        let memory = EntityMemory::new();
        memory.save("k1", msg("My name is Bob")).await.unwrap();
        memory.clear("k1").await.unwrap();
        assert_eq!(memory.size("k1").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_entity_memory_with_custom_keywords() {
        let memory = EntityMemory::new().with_keywords(vec!["age".to_string(), "color".to_string()]);
        memory.save("k1", msg("age 25")).await.unwrap();
        memory.save("k1", msg("color blue")).await.unwrap();

        assert!(memory.size("k1").await.unwrap() > 0);
    }
}