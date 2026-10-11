---
title: Agent Trait
---

# Agent Trait · Agent trait

核心抽象。每个自主单元都实现这个 trait。

## 定义

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent>;

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: ResumeInfo,
    ) -> AgentResult<AgentEvent>;
}
```

## 类型

```rust
pub struct AgentInput {
    pub messages: Vec<Message>,
    pub enable_streaming: bool,
}

pub struct AgentEvent {
    pub agent_name: String,
    pub output: AgentOutput,
    pub error: Option<AgentError>,
}

pub struct AgentOutput {
    pub message: Option<Message>,
    pub action: Option<AgentAction>,
}

pub struct AgentAction {
    pub exit: bool,
    pub transfer_to: Option<String>,
    pub interrupted: Option<InterruptInfo>,
}

pub struct ResumeInfo {
    pub user_input: String,
}

pub enum AgentError {
    Agent(String),
    MaxIterationsExceeded,
    ToolError(String),
    InvalidInput(String),
    Interrupted,
}
```

## 相关阅读
