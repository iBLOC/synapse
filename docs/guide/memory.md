# Memory 系统

`synapse::memory` 模块提供了多种 Memory 实现，用于 agent 的状态管理和对话历史记录。

## 概述

Memory 系统借鉴 LangChain 的设计，提供 5 种 Memory 实现：

| 类型 | 说明 | 适用场景 |
|------|------|----------|
| `ConversationBufferMemory` | 简单缓冲区，存储所有消息 | 短对话 |
| `WindowMemory` | 滑动窗口，保留最近 N 条 | 长对话，避免 token 过多 |
| `SummaryMemory` | 自动摘要压缩 | 超长对话 |
| `VectorStoreMemory` | 向量相似度检索 | 语义检索场景 |
| `EntityMemory` | 实体抽取和跟踪 | 实体关系管理 |

## 统一接口

所有 Memory 实现都遵循统一的 `Memory` trait：

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>>;
    async fn save(&self, key: &str, message: Message) -> MemoryResult<()>;
    async fn save_batch(&self, key: &str, messages: Vec<Message>) -> MemoryResult<()>;
    async fn clear(&self, key: &str) -> MemoryResult<()>;
    async fn size(&self, key: &str) -> MemoryResult<usize>;
}
```

## 使用示例

### ConversationBufferMemory

```rust
use synapse::memory::ConversationBufferMemory;
use synapse::memory::Memory;
use synapse::schema::{Message, RoleType, Content};

let memory = ConversationBufferMemory::new();

let msg = Message {
    role: RoleType::User,
    content: Content::Text("Hello".into()),
    name: None,
    tool_call_id: None,
    tool_calls: None,
};

memory.save("user-123", msg).await?;
let history = memory.load("user-123").await?;
```

### WindowMemory

```rust
use synapse::memory::WindowMemory;

let memory = WindowMemory::new(10); // 只保留最近 10 条
// ... 使用方式同 ConversationBufferMemory
```

### SummaryMemory

需要实现 `SummaryFn` trait 来定义摘要逻辑：

```rust
use synapse::memory::{SummaryMemory, SummaryFn, ConcatSummarizer};
use std::sync::Arc;

struct MySummarizer;
#[async_trait]
impl SummaryFn for MySummarizer {
    async fn summarize(
        &self,
        current: Option<&str>,
        new_messages: &[Message],
    ) -> MemoryResult<String> {
        // 调用 LLM 生成摘要
        // ...
        Ok("summary".to_string())
    }
}

let summarizer = Arc::new(MySummarizer);
let memory = SummaryMemory::new(20, summarizer);
// 或使用简单的拼接实现
let memory = SummaryMemory::new(20, Arc::new(ConcatSummarizer));
```

### VectorStoreMemory

需要实现 `EmbedFn` trait：

```rust
use synapse::memory::{VectorStoreMemory, EmbedFn, HashEmbedFn};
use std::sync::Arc;

struct MyEmbedder;
#[async_trait]
impl EmbedFn for MyEmbedder {
    async fn embed(&self, text: &str) -> MemoryResult<Vec<f32>> {
        // 调用 embedding API
        Ok(vec![0.1, 0.2, 0.3])
    }
}

let embedder = Arc::new(MyEmbedder);
let memory = VectorStoreMemory::new(embedder, 5); // top-5

// 基于查询检索
let relevant = memory.load_with_query("user-123", "what is rust?").await?;
```

或使用内置的 `HashEmbedFn`（仅用于测试）：

```rust
let memory = VectorStoreMemory::new(Arc::new(HashEmbedFn::new(64)), 5);
```

### EntityMemory

```rust
use synapse::memory::EntityMemory;

let memory = EntityMemory::new();
memory.save("user-123", msg_with_name).await?;

let history = memory.load("user-123").await?;
// history 包含一个 System 消息，列出已知实体

// 自定义实体关键词
let memory = EntityMemory::new()
    .with_keywords(vec!["age".into(), "color".into()]);
```

## 在 Agent 中使用

```rust
use synapse::memory::{Memory, WindowMemory};
use std::sync::Arc;

struct MyAgent {
    memory: Arc<WindowMemory>,
}

impl MyAgent {
    async fn chat(&self, user_id: &str, user_msg: Message) -> Result<Message> {
        // 加载历史
        let history = self.memory.load(user_id).await?;
        
        // 构建 prompt（历史 + 当前消息）
        let mut messages = history;
        messages.push(user_msg.clone());
        
        // 调用 LLM
        let response = call_llm(messages).await?;
        
        // 保存到 memory
        self.memory.save(user_id, user_msg).await?;
        self.memory.save(user_id, response.clone()).await?;
        
        Ok(response)
    }
}
```

## 测试

```bash
cargo test --lib memory
```

Memory 模块包含 19 个测试，覆盖所有实现的核心功能。

## 性能考虑

| 类型 | 性能特征 | 适用规模 |
|------|----------|----------|
| ConversationBufferMemory | O(1) 写入，O(n) 读取 | <1000 条消息 |
| WindowMemory | O(1) 写入，O(n) 读取 | 任意规模（自动截断） |
| SummaryMemory | O(n) 摘要调用 | 任意规模（可压缩） |
| VectorStoreMemory | O(n) 相似度计算 | <10K 条消息 |
| EntityMemory | O(n) 关键词匹配 | <10K 条消息 |

## 与 LangChain 的对应

| LangChain | Synapse |
|-----------|---------|
| `ConversationBufferMemory` | `ConversationBufferMemory` |
| `ConversationBufferWindowMemory` | `WindowMemory` |
| `ConversationSummaryMemory` | `SummaryMemory` |
| `VectorStoreRetrieverMemory` | `VectorStoreMemory` |
| `ConversationEntityMemory` | `EntityMemory` |

## 扩展自定义 Memory

实现 `Memory` trait 即可创建自定义 Memory：

```rust
use async_trait::async_trait;
use synapse::memory::{Memory, MemoryResult};
use synapse::schema::Message;

pub struct RedisMemory {
    client: redis::Client,
}

#[async_trait]
impl Memory for RedisMemory {
    async fn load(&self, key: &str) -> MemoryResult<Vec<Message>> {
        // 从 Redis 加载
        // ...
    }
    
    async fn save(&self, key: &str, message: Message) -> MemoryResult<()> {
        // 保存到 Redis
        // ...
    }
    
    async fn clear(&self, key: &str) -> MemoryResult<()> {
        // 从 Redis 删除
        // ...
    }
}
```

## 参考

- [LangChain Memory](https://python.langchain.com/docs/modules/memory/)
- [LangChain VectorStoreRetrieverMemory](https://python.langchain.com/docs/modules/memory/types/vectorstore_retriever_memory)