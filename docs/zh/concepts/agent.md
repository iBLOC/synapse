---
title: Agent
---

# Agent（智能体）

`Agent` trait 是 Synapse 的核心抽象。每一个自治单元 —— ReAct agent、Router、Orchestrator、Handoff、自定义逻辑 —— 都实现这个 trait。

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

## 输入/输出

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
    pub transfer_to: Option<String>,  // for HandoffSupervisor
    pub interrupted: Option<InterruptInfo>,
}
```

## 内置 Agent 实现

| Agent | 用例 | 运行循环 |
|-------|----------|----------|
| `ChatModelAgent<M, T>` | 单个 LLM + 工具 | ReAct 循环 |
| `GraphReActAgent` | 基于图的编排 | 图遍历 |
| `RouterSupervisor` | 多 agent 路由 | ReAct 循环 + sub-agent 工具 |
| `OrchestratorAgent` | 任务分解 | ReAct 循环 + 并行 tool call |
| `HandoffSupervisor` | 多角色对话 | 顺序调用 sub-agent |
| `HybridSupervisor` | 路由 + 移交 | ReAct + 强制移交 |

## 实现自定义 Agent

```rust
use async_trait::async_trait;
use synapse::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentOutput, AgentResult, ResumeInfo,
};

struct MyAgent {
    name: String,
    description: String,
}

#[async_trait]
impl Agent for MyAgent {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.description.clone() }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant("Hello!")),
                action: None,
            },
            error: None,
        })
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not supported".into()))
    }
}
```

## Agent 组合

`Agent` 是一个 trait，因此 agent 可以组合：

```rust
let outer: Arc<dyn Agent> = Arc::new(my_outer_agent);
let inner: Arc<dyn Agent> = Arc::new(my_inner_agent);
outer.run(input).await?;
```

MultiAgent 模式正是利用这一点实现层级化组合。
