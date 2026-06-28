# Synapse MultiAgent (Conversational Handoff Pattern) 设计 Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-28

## 背景与目标

### 现状

Synapse 已有：
- `RouterSupervisor` (multiagent/router.rs) — LLM 选择 sub-agent
- `OrchestratorAgent` (multiagent/orchestrator.rs) — LLM 分解任务并并行 dispatch
- `Agent` trait — `AgentEvent.output.action: Option<AgentAction>` 已经有 `transfer_to: Option<String>` 字段！
- `AgentAction { exit, transfer_to, interrupted }` — 已定义但未使用

### Handoff vs 其他 MultiAgent 模式

| 模式 | 谁决定下一步 | 协调信号 |
|------|--------------|----------|
| Router/Supervisor | 顶层 LLM (host) | tool call |
| Host/Orchestrator | 顶层 LLM (host) | tool call (parallel) |
| **Conversational Handoff** | **子 agent 自己** | **`event.action.transfer_to`** |

### 目标

实现 **Conversational Handoff** 模式：
- Sub-agent 可以"移交"对话给其他 agent
- 通过 `event.action.transfer_to` 字段
- HandoffSupervisor 维护 message history，按 transfer 信号路由
- 无限循环由 `max_handoffs` 限制

## 设计

### 架构

```
┌────────────────────────────────────────────────────────────┐
│  Runner                                                    │
│   │                                                        │
│   ▼                                                        │
│  HandoffSupervisor (coordinator, no LLM)                    │
│   │  agents: { "billing": agent, "refund": agent, ... }    │
│   │  entry_point: "billing"                                │
│   │  max_handoffs: 5                                       │
│   │                                                        │
│   │  iteration 0:                                          │
│   │    agent = agents["billing"]                           │
│   │    event = agent.run(messages)                         │
│   │    if event.action.transfer_to == "refund":            │
│   │      current = "refund"                                │
│   │      messages.push(event.message)                      │
│   │    else:                                               │
│   │      return event  ← final                             │
│   │                                                        │
│   │  iteration 1:                                          │
│   │    agent = agents["refund"]                            │
│   │    event = agent.run(messages)  ← sees A's history    │
│   │    if event.action.transfer_to == "tech":              │
│   │      current = "tech"                                  │
│   │    else:                                               │
│   │      return event  ← final                             │
│   │                                                        │
│   │  ...                                                   │
│   ▼                                                        │
│  Final AgentEvent (last agent's output)                    │
└────────────────────────────────────────────────────────────┘
```

### 组件

#### HandoffSupervisor

```rust
// src/adk/multiagent/handoff.rs
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentOutput, AgentResult, ResumeInfo,
};
use crate::schema::Message;

pub struct HandoffSupervisor {
    name: String,
    description: String,
    agents: HashMap<String, Arc<dyn Agent>>,
    entry_point: String,
    max_handoffs: usize,
}

impl HandoffSupervisor {
    pub fn builder(name: impl Into<String>) -> HandoffSupervisorBuilder {
        HandoffSupervisorBuilder::new(name)
    }
}

#[async_trait]
impl Agent for HandoffSupervisor {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.description.clone() }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let mut current = self.entry_point.clone();
        let mut messages = input.messages;
        let mut handoff_count = 0;
        let mut last_event: Option<AgentEvent> = None;

        loop {
            let agent = self.agents.get(&current).ok_or_else(|| {
                AgentError::Agent(format!("handoff target '{}' not found", current))
            })?;

            let event = agent.run(AgentInput {
                messages: messages.clone(),
                enable_streaming: false,
            }).await?;

            // Check for handoff signal
            let transfer_target = event
                .output
                .action
                .as_ref()
                .and_then(|a| a.transfer_to.clone());

            match transfer_target {
                Some(target) => {
                    handoff_count += 1;
                    if handoff_count > self.max_handoffs {
                        return Err(AgentError::Agent(
                            "max handoffs exceeded".to_string(),
                        ));
                    }
                    if let Some(msg) = &event.output.message {
                        messages.push(msg.clone());
                    }
                    last_event = Some(event);
                    current = target;
                }
                None => {
                    return Ok(event);
                }
            }
        }
    }

    async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct HandoffSupervisorBuilder {
    name: String,
    description: String,
    agents: HashMap<String, Arc<dyn Agent>>,
    entry_point: Option<String>,
    max_handoffs: usize,
}

impl HandoffSupervisorBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: "Handoff supervisor".to_string(),
            agents: HashMap::new(),
            entry_point: None,
            max_handoffs: 5,
        }
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn add_agent(mut self, key: impl Into<String>, agent: Arc<dyn Agent>) -> Self {
        self.agents.insert(key.into(), agent);
        self
    }

    pub fn entry_point(mut self, name: impl Into<String>) -> Self {
        self.entry_point = Some(name.into());
        self
    }

    pub fn max_handoffs(mut self, n: usize) -> Self {
        self.max_handoffs = n;
        self
    }

    pub fn build(self) -> HandoffSupervisor {
        let entry = self.entry_point
            .or_else(|| self.agents.keys().next().cloned())
            .expect("at least one agent must be added");
        HandoffSupervisor {
            name: self.name,
            description: self.description,
            agents: self.agents,
            entry_point: entry,
            max_handoffs: self.max_handoffs,
        }
    }
}
```

### 关键设计决策

1. **HandoffSupervisor 不是 LLM agent** — 简单协调器，无 ReAct loop
2. **Sub-agent 决定 handoff** — 通过 `event.action.transfer_to` 字段（已有 schema）
3. **消息历史累积** — 每个 sub-agent 看到之前所有对话
4. **max_handoffs 防死循环** — 默认 5，可配置
5. **Sub-agent 应该是 LLM agent** — 它们自己决定何时 handoff（生产场景）
6. **测试用 mock** — `MockHandoffAgent` 返回预设 transfer_to

### 与现有 AgentEvent 的关系

`AgentEvent` 已有 `output.action: Option<AgentAction>`，`AgentAction` 已有 `transfer_to: Option<String>` 字段。本 spec **不修改 schema**，只使用已有字段。

测试用 `MockHandoffAgent` 直接构造 `AgentEvent` 时填充 `transfer_to`：

```rust
let event = AgentEvent {
    agent_name: "billing".into(),
    output: AgentOutput {
        message: Some(Message::assistant("This is actually a refund")),
        action: Some(AgentAction {
            exit: false,
            transfer_to: Some("refund".into()),
            interrupted: None,
        }),
    },
    error: None,
};
```

### 文件结构

```
src/adk/multiagent/
├── mod.rs                (添加 HandoffSupervisor re-export)
├── agent_tool.rs         (复用)
├── router.rs             (复用)
├── orchestrator.rs       (复用)
└── handoff.rs            (NEW)

tests/handoff_test.rs     (NEW — 5 integration tests)
```

### 公开 API

```rust
// src/adk/multiagent/mod.rs
pub use handoff::{HandoffSupervisor, HandoffSupervisorBuilder};

// 用户使用（生产场景）
let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .add_agent("tech_support", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();

let result = supervisor.run(input).await?;
// Sub-agents 自己决定何时 handoff
```

### 测试

#### 测试矩阵

| # | 测试名 | 场景 | 验证点 |
|---|--------|------|--------|
| 1 | `test_handoff_single_transfer` | A → B（1 次 handoff） | A 被调用，B 被调用；最终输出 = B |
| 2 | `test_handoff_chain` | A → B → C | 三个 agent 按顺序被调用；最终输出 = C |
| 3 | `test_handoff_no_transfer` | A 直接回答 | 只有 A 被调用；最终输出 = A |
| 4 | `test_handoff_max_limit_exceeded` | 无限循环 (A→A→A...) | 返回 AgentError |
| 5 | `test_handoff_preserves_messages` | A → B 时 B 看到 A 的 message | B 收到 messages 包含 A 的回答 |

#### Mock helper

```rust
// 每个 MockHandoffAgent 记录收到的 messages，返回预设 event（含 transfer_to）
struct MockHandoffAgent {
    name: String,
    responses: Vec<(String, Option<String>)>,  // (text, transfer_to)
    index: AtomicUsize,
    received_messages: Arc<Mutex<Vec<Message>>>,
}

impl Agent for MockHandoffAgent {
    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        self.received_messages.lock().unwrap().extend(input.messages);
        let (text, transfer) = &self.responses[self.index.fetch_add(1, Ordering::SeqCst)];
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant(text)),
                action: Some(AgentAction {
                    exit: false,
                    transfer_to: transfer.clone(),
                    interrupted: None,
                }),
            },
            error: None,
        })
    }
}
```

### 与现有 MultiAgent 模式的对比

| 模式 | 协调器 | 协调信号 | 决策者 | 适用场景 |
|------|--------|----------|--------|----------|
| Router/Supervisor | LLM | tool call | 顶层 LLM | 单任务路由 |
| Host/Orchestrator | LLM | tool call (parallel) | 顶层 LLM | 多视角并行 |
| **HandoffSupervisor** | **无 LLM** | **action.transfer_to** | **子 agent** | 多角色对话切换 |

### 风险与权衡

| 风险 | 缓解 |
|------|------|
| Sub-agent 不设置 transfer_to | 显式 schema；测试覆盖 no_transfer 场景 |
| 死循环（A→A→A） | max_handoffs 默认 5 |
| entry_point 不存在 | builder 强制必须 add_agent；build 时校验 |
| Handoff 目标不存在 | runtime 错误（清晰） |
| Message 累积过长 | 未来加 message 截断；目前交给用户控制 |

### 实施分解

| Step | 内容 | 估时 |
|------|------|------|
| 1 | handoff.rs + HandoffSupervisor + Builder | 30min |
| 2 | mod.rs re-export | 5min |
| 3 | 5 integration tests (含 MockHandoffAgent helper) | 1h |
| 4 | 跑全套测试 + 修 edge cases | 15min |
| 5 | commit + merge | 10min |
| **总计** | | **~2h** |

## 后续工作（不在本 spec 范围）

- **混合 Router + Handoff**（同一 supervisor 内部用两种模式）
- **Transfer tool**（给 LLM-based sub-agent 加 transfer 工具）
- **Message 截断**（防止历史过长）
- **Handoff chain 可视化**（debug 工具）
