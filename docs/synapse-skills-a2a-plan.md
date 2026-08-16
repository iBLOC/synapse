# Synapse Skills + A2A 协议实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan.

**Goal:** 在 Synapse 智能体框架中实现 Skills 系统和 A2A 协议支持

**Architecture:** 
- Skills: 基于关键词/模式激活的可扩展技能系统，支持 Agent 调用
- A2A: Google A2A 协议实现，支持 Agent 间 JSON-RPC 通信

**Tech Stack:** Rust, tokio, serde, async-trait

---

## 文件结构

```
src/
├── skills/                    # 新增: Skills 系统
│   ├── mod.rs               # SkillManager, Skill trait
│   ├── context.rs           # SkillContext
│   └── builtin/             # 内置 Skills
│       └── mod.rs
├── a2a/                      # 新增: A2A 协议
│   ├── mod.rs               # A2A Client/Server
│   ├── protocol.rs          # JSON-RPC 消息定义
│   ├── card.rs              # Agent Card (发现机制)
│   └── transport.rs          # HTTP/WebSocket 传输
```

---

## Chunk 1: Skills 系统基础

### Task 1.1: Skill Trait 定义

**Files:**
- Create: `src/skills/mod.rs`

- [ ] **Step 1: 定义 Skill Trait**

```rust
use async_trait::async_trait;

pub trait Skill: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    
    fn keywords(&self) -> Vec<String>;
    fn patterns(&self) -> Vec<String>;
    fn max_context_tokens(&self) -> Option<usize>;
    
    async fn execute(&self, ctx: &SkillContext, input: &str) -> Result<String, SkillError>;
}

pub struct SkillContext {
    pub messages: Vec<crate::schema::Message>,
    pub state: std::collections::HashMap<String, serde_json::Value>,
    pub tools: Vec<Arc<dyn crate::components::tool::BaseTool>>,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("execution error: {0}")]
    Execution(String),
    #[error("not found: {0}")]
    NotFound(String),
}
```

### Task 1.2: SkillManager

**Files:**
- Modify: `src/skills/mod.rs`

- [ ] **Step 1: 实现 SkillManager**

```rust
pub struct SkillManager {
    skills: std::collections::HashMap<String, Arc<dyn Skill>>,
}

impl SkillManager {
    pub fn new() -> Self;
    pub fn register(&mut self, skill: Arc<dyn Skill>);
    pub fn find_skill(&self, input: &str) -> Option<Arc<dyn Skill>>;
    pub async fn execute(&self, skill_name: &str, ctx: &SkillContext, input: &str) -> Result<String, SkillError>;
}
```

---

## Chunk 2: A2A 协议基础

### Task 2.1: A2A 消息协议

**Files:**
- Create: `src/a2a/protocol.rs`

- [ ] **Step 1: 定义 JSON-RPC 消息**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
```

### Task 2.2: A2A 消息类型

**Files:**
- Modify: `src/a2a/protocol.rs`

- [ ] **Step 1: 定义 A2A 任务消息**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum A2AMethod {
    #[serde(rename = "tasks/send")]
    TasksSend(TasksSendParams),
    #[serde(rename = "tasks/get")]
    TasksGet(TasksGetParams),
    #[serde(rename = "tasks/cancel")]
    TasksCancel(TasksCancelParams),
    #[serde(rename = "tasks/pushNotificationConfig")]
    PushNotificationConfig(PushNotificationConfigParams),
    #[serde(rename = "agents/publish")]
    AgentsPublish(AgentsPublishParams),
    #[serde(rename = "agents/get")]
    AgentsGet(AgentsGetParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksSendParams {
    pub id: String,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification: Option<PushNotificationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub parts: Vec<MessagePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}
```

### Task 2.3: Agent Card

**Files:**
- Create: `src/a2a/card.rs`

- [ ] **Step 1: 实现 Agent Card**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    pub provider: Option<Provider>,
    pub capabilities: Capabilities,
    pub authentication: Option<Authentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<SkillReference>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
}
```

### Task 2.4: A2A Client

**Files:**
- Create: `src/a2a/mod.rs`

- [ ] **Step 1: 实现 A2AClient**

```rust
pub struct A2AClient {
    client: reqwest::Client,
    agent_url: String,
}

impl A2AClient {
    pub fn new(agent_url: impl Into<String>) -> Self;
    
    pub async fn send_task(&self, task_id: &str, message: Message) -> Result<Task, A2AError>;
    pub async fn get_task(&self, task_id: &str) -> Result<Task, A2AError>;
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), A2AError>;
}
```

### Task 2.5: A2A Server

**Files:**
- Modify: `src/a2a/mod.rs`

- [ ] **Step 1: 实现 A2AServer**

```rust
pub struct A2AServer {
    agent: Arc<dyn crate::adk::agent::Agent>,
    card: AgentCard,
}

impl A2AServer {
    pub fn new(agent: Arc<dyn crate::adk::agent::Agent>, card: AgentCard) -> Self;
    
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse;
}
```

---

## Chunk 3: Skills 与 Agent 集成

### Task 3.1: Agent 集成 Skills

**Files:**
- Modify: `src/adk/react.rs`

- [ ] **Step 1: 在 ChatModelAgent 中集成 Skills**

```rust
pub struct ChatModelAgent<M: BaseChatModel, T: InvokableTool> {
    // ... existing fields
    skill_manager: Option<Arc<crate::skills::SkillManager>>,
}

impl<M: BaseChatModel + 'static, T: InvokableTool + 'static> ChatModelAgent<M, T> {
    pub fn with_skills(mut self, manager: Arc<crate::skills::SkillManager>) -> Self {
        self.skill_manager = Some(manager);
        self
    }
    
    async fn maybe_execute_skill(&self, input: &str) -> Option<String> {
        // 检查是否匹配 skill 激活条件
    }
}
```

---

## 执行指南

1. 先实现 Skills 系统基础
2. 实现 A2A 协议消息类型
3. 实现 A2A Client/Server
4. 集成到 Agent 中
