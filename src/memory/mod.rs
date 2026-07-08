//! Memory 系统（LangChain 风格）
//!
//! 提供多种 Memory 实现用于 agent 的状态管理：
//! - `ConversationBufferMemory` - 简单的对话缓冲区
//! - `WindowMemory` - 滑动窗口记忆
//! - `SummaryMemory` - 摘要记忆
//! - `VectorStoreMemory` - 向量存储记忆
//!
//! 所有 Memory 实现统一的 `Memory` trait。

pub mod buffer;
pub mod window;
pub mod summary;
pub mod vector;
pub mod entity;

pub use buffer::ConversationBufferMemory;
pub use entity::EntityMemory;
pub use summary::SummaryMemory;
pub use vector::VectorStoreMemory;
pub use window::WindowMemory;

use std::sync::Arc;
use thiserror::Error;

use crate::schema::Message;

/// Memory 错误
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("memory not found: {0}")]
    NotFound(String),

    #[error("memory store error: {0}")]
    Store(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory 统一 trait
///
/// 所有 Memory 实现都提供：
/// - `load`: 加载历史消息
/// - `save`: 保存消息
/// - `clear`: 清除历史
#[async_trait::async_trait]
pub trait Memory: Send + Sync {
    /// 加载指定 key 的记忆
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>>;

    /// 保存消息到记忆
    async fn save(&self, key: &str, message: Message) -> MemoryResult<()>;

    /// 批量保存消息
    async fn save_batch(&self, key: &str, messages: Vec<Message>) -> MemoryResult<()> {
        for msg in messages {
            self.save(key, msg).await?;
        }
        Ok(())
    }

    /// 清除记忆
    async fn clear(&self, key: &str) -> MemoryResult<()>;

    /// 获取记忆大小
    async fn size(&self, key: &str) -> MemoryResult<usize> {
        Ok(self.load(key).await?.len())
    }
}

/// Memory 共享类型
pub type SharedMemory = Arc<dyn Memory>;

/// Memory 键 - 用于区分不同的记忆上下文
pub type MemoryKey = String;
