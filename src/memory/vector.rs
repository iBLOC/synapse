use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::{Memory, MemoryError, MemoryResult};
use crate::schema::Message;

/// 向量存储 Memory
///
/// 使用向量相似度检索最相关的历史消息。
/// 适合长对话或需要语义检索的场景。
pub struct VectorStoreMemory {
    store: Arc<RwLock<DashMap<String, Vec<StoredMemory>>>>,
    embedding: Arc<dyn EmbedFn>,
    k: usize,
}

#[derive(Clone)]
struct StoredMemory {
    message: Message,
    embedding: Vec<f32>,
}

/// Embedding 函数 trait
#[async_trait::async_trait]
pub trait EmbedFn: Send + Sync {
    async fn embed(&self, text: &str) -> MemoryResult<Vec<f32>>;
}

impl VectorStoreMemory {
    pub fn new(embedding: Arc<dyn EmbedFn>, k: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(DashMap::new())),
            embedding,
            k: k.max(1),
        }
    }

    pub fn k(&self) -> usize {
        self.k
    }

    /// 计算两个向量的余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// 提取消息的文本内容
    fn extract_text(message: &Message) -> String {
        match &message.content {
            crate::schema::Content::Text(t) => t.clone(),
            _ => String::new(),
        }
    }
}

#[async_trait]
impl Memory for VectorStoreMemory {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>> {
        self.load_with_query(key, "").await
    }

    async fn save(&self, key: &str, message: Message) -> MemoryResult<()> {
        let text = Self::extract_text(&message);
        if text.is_empty() {
            return Ok(());
        }
        let embedding = self.embedding.embed(&text).await?;

        let mut store = self.store.write().await;
        if !store.contains_key(key) {
            store.insert(key.to_string(), Vec::new());
        }
        let mut entry = store.get_mut(key).unwrap();
        entry.push(StoredMemory { message, embedding });
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

impl VectorStoreMemory {
    /// 使用查询检索最相关的 top-k 消息
    pub async fn load_with_query(&self, key: &str, query: &str) -> MemoryResult<Vec<Message>> {
        if query.is_empty() {
            let store = self.store.read().await;
            let messages: Vec<Message> = store
                .get(key)
                .map(|e| e.value().iter().map(|s| s.message.clone()).collect())
                .unwrap_or_default();
            return Ok(messages);
        }

        let query_embedding = self.embedding.embed(query).await?;
        let store = self.store.read().await;

        let Some(entries) = store.get(key) else {
            return Ok(Vec::new());
        };

        let mut scored: Vec<(f32, &StoredMemory)> = entries
            .value()
            .iter()
            .map(|stored| (Self::cosine_similarity(&query_embedding, &stored.embedding), stored))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored
            .into_iter()
            .take(self.k)
            .map(|(_, stored)| stored.message.clone())
            .collect())
    }
}

/// 简单的 Embedding 函数 - 基于字符哈希的伪 embedding
///
/// 仅用于测试和演示。生产环境应使用真实的 embedding provider。
pub struct HashEmbedFn {
    dim: usize,
}

impl HashEmbedFn {
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(8) }
    }
}

#[async_trait]
impl EmbedFn for HashEmbedFn {
    async fn embed(&self, text: &str) -> MemoryResult<Vec<f32>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut result = vec![0.0; self.dim];
        for (i, c) in text.chars().enumerate() {
            let mut hasher = DefaultHasher::new();
            c.hash(&mut hasher);
            let h = hasher.finish();
            let idx = (h as usize) % self.dim;
            result[idx] += 1.0;
        }
        // 归一化
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in result.iter_mut() {
                *x /= norm;
            }
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
    async fn test_vector_memory_basic() {
        let embed = Arc::new(HashEmbedFn::new(64));
        let memory = VectorStoreMemory::new(embed, 2);

        memory.save("k1", msg("hello world")).await.unwrap();
        memory.save("k1", msg("goodbye world")).await.unwrap();

        let msgs = memory.load("k1").await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn test_vector_memory_query() {
        let embed = Arc::new(HashEmbedFn::new(64));
        let memory = VectorStoreMemory::new(embed, 2);

        memory.save("k1", msg("hello world")).await.unwrap();
        memory.save("k1", msg("rust programming")).await.unwrap();
        memory.save("k1", msg("python coding")).await.unwrap();

        let msgs = memory.load_with_query("k1", "hello").await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn test_vector_memory_clear() {
        let embed = Arc::new(HashEmbedFn::new(64));
        let memory = VectorStoreMemory::new(embed, 2);
        memory.save("k1", msg("test")).await.unwrap();
        memory.clear("k1").await.unwrap();
        assert_eq!(memory.size("k1").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        let sim = VectorStoreMemory::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0];
        let sim2 = VectorStoreMemory::cosine_similarity(&a, &c);
        assert!(sim2.abs() < 0.001);
    }
}