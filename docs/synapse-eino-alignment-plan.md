# Synapse 功能对齐 Eino 实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan.

**Goal:** 实现 Rust 版本的智能体开发框架，功能与 Go 版 Eino 完全对齐

**Architecture:** 参考 Eino 架构，分层实现：Schema -> Components -> Compose -> ADK -> Callbacks

**Tech Stack:** Rust, tokio, async-trait, serde, thiserror

---

## 文件结构

```
src/
├── schema/
│   └── mod.rs          # Message, RoleType, ToolCall, ToolInfo, StreamReader (已存在)
├── components/
│   ├── mod.rs          # 组件导出
│   ├── model/          # LLM 模型接口 (已存在)
│   ├── tool/           # 工具接口 (已存在)
│   ├── retriever/     # 检索接口 (已存在)
│   ├── embedding/      # 向量嵌入 (已存在)
│   ├── indexer/        # 文档索引
│   ├── document/       # 文档处理
│   └── prompt/         # Prompt 模板
├── compose/
│   ├── mod.rs          # 导出
│   ├── graph/          # DAG 编排 (已存在基础)
│   ├── chain/          # 链式调用 (已存在)
│   ├── node/           # 节点抽象 (已存在)
│   ├── runnable/      # 可运行接口
│   └── stream.rs       # 流处理
├── adk/
│   ├── mod.rs          # 导出
│   ├── agent/          # Agent 接口 (已存在)
│   ├── runner/         # 运行器 (已存在)
│   ├── state/         # 状态管理 (已存在)
│   ├── react.rs        # ReAct Agent
│   └── prebuilt/       # 预置 Agent
├── callbacks/
│   └── mod.rs          # 回调系统 (已存在基础)
└── utils/
    └── mod.rs          # 工具 (已存在)
```

---

## Chunk 1: Schema 扩展 + Stream 处理

### Task 1.1: 完善 StreamReader

**Files:**
- Modify: `src/components/model/mod.rs`

- [ ] **Step 1: 添加 StreamReader 完整实现**

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct StreamReader<T> {
    receiver: tokio::sync::mpsc::Receiver<Result<T, ModelError>>,
}

impl<T> StreamReader<T> {
    pub fn new() -> Self {
        todo!()
    }
    
    pub async fn next(&mut self) -> Option<Result<T, ModelError>> {
        todo!()
    }
    
    pub fn boxed(self) -> BoxStreamReader<T> 
    where T: 'static {
        todo!()
    }
}

pub type BoxStreamReader<T> = Pin<Box<dyn Stream<Item = Result<T, ModelError>> + Send>>;
pub type StreamItem<T> = Result<T, ModelError>;
```

- [ ] **Step 2: 测试 StreamReader**

Run: `cargo build`
Expected: 编译通过

---

## Chunk 2: Components 完整接口

### Task 2.1: Indexer 组件

**Files:**
- Create: `src/components/indexer/mod.rs`

- [ ] **Step 1: 实现 Indexer 接口**

```rust
use async_trait::async_trait;
use crate::components::retriever::Document;

pub type IndexerResult<T> = Result<T, IndexerError>;

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("index error: {0}")]
    Index(String),
}

#[async_trait]
pub trait Indexer: Send + Sync {
    async fn index(&self, documents: Vec<Document>) -> IndexerResult<()>;
    
    async fn index_from_reader(&self, reader: impl tokio::ioAsyncRead + Send) -> IndexerResult<()>;
}
```

### Task 2.2: Document 组件

**Files:**
- Create: `src/components/document/mod.rs`

- [ ] **Step 1: 实现 Document Parser 接口**

```rust
use async_trait::async_trait;

pub type DocumentResult<T> = Result<T, DocumentError>;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported format")]
    UnsupportedFormat,
}

#[async_trait]
pub trait DocumentParser: Send + Sync {
    async fn parse(&self, content: &[u8], mime_type: &str) -> DocumentResult<ParsedDocument>;
}

pub struct ParsedDocument {
    pub content: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}
```

### Task 2.3: Prompt 组件

**Files:**
- Create: `src/components/prompt/mod.rs`

- [ ] **Step 1: 实现 ChatTemplate 接口**

```rust
use async_trait::async_trait;
use crate::schema::Message;

pub type PromptResult<T> = Result<T, PromptError>;

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("template error: {0}")]
    Template(String),
}

#[async_trait]
pub trait ChatTemplate: Send + Sync {
    fn format(&self, messages: Vec<Message>) -> PromptResult<String>;
    
    fn get_template(&self) -> &str;
    
    fn get_variables(&self) -> Vec<String>;
}
```

---

## Chunk 3: Compose 图编排

### Task 3.1: Runnable 接口

**Files:**
- Create: `src/compose/runnable/mod.rs`

- [ ] **Step 1: 实现 Runnable trait**

```rust
use async_trait::async_trait;
use crate::components::model::StreamReader;

#[async_trait]
pub trait Runnable<I, O>: Send + Sync {
    async fn invoke(&self, input: I) -> Result<O, String>;
    
    async fn stream(&self, input: I) -> Result<StreamReader<O>, String>;
    
    fn name(&self) -> String;
}

pub struct RunnableMap<I, O> {
    runnable: Box<dyn Runnable<I, O>>,
}
```

### Task 3.2: Graph 执行引擎

**Files:**
- Modify: `src/compose/graph/mod.rs`

- [ ] **Step 1: 实现 Graph 编译和执行**

```rust
impl<I: 'static, O: 'static> Graph<I, O> {
    pub fn compile(self) -> Result<CompiledGraph<I, O>, String> {
        todo!("实现图编译验证")
    }
    
    pub fn add_chat_model_node(&mut self, name: &str, model: Arc<dyn crate::components::model::BaseChatModel>) -> &mut Self {
        todo!()
    }
    
    pub fn add_tool_node(&mut self, name: &str, tool: Arc<dyn crate::components::tool::BaseTool>) -> &mut Self {
        todo!()
    }
    
    pub fn add_retriever_node(&mut self, name: &str, retriever: Arc<dyn crate::components::retriever::Retriever>) -> &mut Self {
        todo!()
    }
}
```

### Task 3.3: Stream 处理

**Files:**
- Create: `src/compose/stream.rs`

- [ ] **Step 1: 实现流处理工具**

```rust
use crate::components::model::StreamReader;

pub fn concat<T>(readers: Vec<StreamReader<T>>) -> StreamReader<T> {
    todo!("合并多个流")
}

pub fn merge<T>(readers: Vec<StreamReader<T>>) -> StreamReader<T> {
    todo!("并行读取多个流")
}

pub fn box_stream<T>(reader: StreamReader<T>) -> BoxStreamReader<T> {
    todo!("装箱流")
}
```

---

## Chunk 4: ADK Agent

### Task 4.1: ReAct Agent

**Files:**
- Create: `src/adk/react.rs`

- [ ] **Step 1: 实现 ReAct Agent**

```rust
use async_trait::async_trait;
use crate::adk::agent::{Agent, AgentInput, AgentEvent, AgentError};
use crate::adk::state::AgentState;
use crate::components::model::BaseChatModel;
use crate::components::tool::BaseTool;

pub struct ChatModelAgent {
    name: String,
    description: String,
    model: Arc<dyn BaseChatModel>,
    tools: Vec<Arc<dyn BaseTool>>,
    max_iterations: usize,
}

impl ChatModelAgent {
    pub fn new(config: ChatModelAgentConfig) -> Self {
        todo!()
    }
}

pub struct ChatModelAgentConfig {
    pub name: String,
    pub description: String,
    pub model: Arc<dyn BaseChatModel>,
    pub tools: Vec<Arc<dyn BaseTool>>,
    pub max_iterations: Option<usize>,
}
```

### Task 4.2: Runner 完善

**Files:**
- Modify: `src/adk/runner/mod.rs`

- [ ] **Step 1: 实现 Runner 完整功能**

```rust
impl Runner {
    pub async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        self.agent.run(input).await
    }
    
    pub fn stream(&self, input: AgentInput) -> impl Stream<Item = Result<AgentEvent, AgentError>> {
        todo!("流式返回")
    }
}
```

### Task 4.3: Deep Agent

**Files:**
- Create: `src/adk/prebuilt/deep.rs`

- [ ] **Step 1: 实现 Deep Agent (多 Agent 编排)**

```rust
pub struct DeepAgent {
    name: String,
    sub_agents: Vec<Box<dyn Agent>>,
    tools: Vec<Arc<dyn BaseTool>>,
}
```

---

## Chunk 5: Callbacks 完善

### Task 5.1: 完整 Callback 系统

**Files:**
- Modify: `src/callbacks/mod.rs`

- [ ] **Step 1: 添加完整回调点**

```rust
pub trait CallbackHandler: Send + Sync {
    fn on_start(&self, ctx: &CallbackContext) -> CallbackResult;
    fn on_end(&self, ctx: &CallbackContext, result: &CallbackResult) -> CallbackResult;
    fn on_error(&self, ctx: &CallbackContext, error: &CallbackError) -> CallbackResult;
    fn on_chat_model_start(&self, ctx: &CallbackContext, messages: &[Message]) -> CallbackResult;
    fn on_chat_model_end(&self, ctx: &CallbackContext, response: &Message) -> CallbackResult;
    fn on_tool_start(&self, ctx: &CallbackContext, tool_call: &ToolCall) -> CallbackResult;
    fn on_tool_end(&self, ctx: &CallbackContext, result: &str) -> CallbackResult;
}
```

---

## Chunk 6: 实际 LLM 实现

### Task 6.1: OpenAI 实现

**Files:**
- Create: `src/components/model/openai.rs`

- [ ] **Step 1: 实现 OpenAI ChatModel**

```rust
pub struct OpenAIChatModel {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIChatModel {
    pub fn new(config: OpenAIConfig) -> Self {
        todo!()
    }
}

pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}
```

---

## 执行指南

1. 依次实现每个 Chunk
2. 每个 Task 需要写测试
3. 运行 `cargo build` 验证
4. 完成后可选择创建 examples/ 演示用法
