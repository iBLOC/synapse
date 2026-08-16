use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryResult};
use crate::schema::Message;

/// 摘要 Memory
///
/// 当消息数量超过阈值时，自动调用摘要函数压缩历史。
/// 适合超长对话场景。
pub struct SummaryMemory {
    store: Arc<RwLock<DashMap<String, SummaryState>>>,
    max_messages: usize,
    summarizer: Arc<dyn SummaryFn>,
}

struct SummaryState {
    summary: Option<String>,
    messages: Vec<Message>,
}

/// 摘要函数签名
#[async_trait::async_trait]
pub trait SummaryFn: Send + Sync {
    async fn summarize(&self, current_summary: Option<&str>, new_messages: &[Message]) -> MemoryResult<String>;
}

impl SummaryMemory {
    pub fn new(max_messages: usize, summarizer: Arc<dyn SummaryFn>) -> Self {
        Self {
            store: Arc::new(RwLock::new(DashMap::new())),
            max_messages: max_messages.max(1),
            summarizer,
        }
    }

    pub fn max_messages(&self) -> usize {
        self.max_messages
    }
}

#[async_trait]
impl Memory for SummaryMemory {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>> {
        let store = self.store.read().await;
        let state = store.get(key);
        let mut result = Vec::new();

        if let Some(state) = state {
            if let Some(summary) = &state.summary {
                result.push(Message {
                    role: crate::schema::RoleType::System,
                    content: crate::schema::Content::Text(format!(
                        "Previous conversation summary: {}",
                        summary
                    )),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
            result.extend_from_slice(&state.messages);
        }

        Ok(result)
    }

async fn save(&self, key: &str, message: Message) -> MemoryResult<()> {
        let mut store = self.store.write().await;
        if !store.contains_key(key) {
            store.insert(
                key.to_string(),
                SummaryState {
                    summary: None,
                    messages: Vec::new(),
                },
            );
        }
        let mut state = store.get_mut(key).unwrap();
        state.messages.push(message);

        if state.messages.len() > self.max_messages {
            let to_summarize: Vec<Message> = state.messages.drain(0..self.max_messages / 2).collect();
            let new_summary = self
                .summarizer
                .summarize(state.summary.as_deref(), &to_summarize)
                .await?;
            state.summary = Some(new_summary);
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
        Ok(store.get(key).map(|e| e.value().messages.len()).unwrap_or(0))
    }
}

impl SummaryMemory {
    /// 获取当前的摘要
    pub async fn get_summary(&self, key: &str) -> MemoryResult<Option<String>> {
        let store = self.store.read().await;
        Ok(store.get(key).and_then(|s| s.summary.clone()))
    }
}

/// 简单的摘要函数 - 直接拼接消息
pub struct ConcatSummarizer;

#[async_trait]
impl SummaryFn for ConcatSummarizer {
    async fn summarize(
        &self,
        current_summary: Option<&str>,
        new_messages: &[Message],
    ) -> MemoryResult<String> {
        let mut result = String::new();
        if let Some(s) = current_summary {
            result.push_str(s);
            result.push_str("\n");
        }
        for msg in new_messages {
            let text = match &msg.content {
                crate::schema::Content::Text(t) => t.clone(),
                _ => String::new(),
            };
            result.push_str(&format!("{:?}: {}\n", msg.role, text));
        }
        Ok(result)
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
    async fn test_summary_memory_basic() {
        let summarizer = Arc::new(ConcatSummarizer);
        let memory = SummaryMemory::new(3, summarizer);

        memory.save("k1", msg("a")).await.unwrap();
        memory.save("k1", msg("b")).await.unwrap();
        memory.save("k1", msg("c")).await.unwrap();

        let msgs = memory.load("k1").await.unwrap();
        assert!(!msgs.is_empty());
    }

    #[tokio::test]
    async fn test_summary_memory_triggers_summary() {
        let summarizer = Arc::new(ConcatSummarizer);
        let memory = SummaryMemory::new(4, summarizer);

        for i in 0..6 {
            memory.save("k1", msg(&format!("msg{}", i))).await.unwrap();
        }

        let summary = memory.get_summary("k1").await.unwrap();
        assert!(summary.is_some());
    }

    #[tokio::test]
    async fn test_summary_memory_clear() {
        let summarizer = Arc::new(ConcatSummarizer);
        let memory = SummaryMemory::new(3, summarizer);
        memory.save("k1", msg("a")).await.unwrap();
        memory.clear("k1").await.unwrap();
        assert!(memory.load("k1").await.unwrap().is_empty());
        assert!(memory.get_summary("k1").await.unwrap().is_none());
    }
}