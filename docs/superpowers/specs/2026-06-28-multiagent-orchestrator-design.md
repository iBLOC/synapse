# Synapse MultiAgent (Host/Orchestrator Pattern) 设计 Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-28

## 背景与目标

### 现状

Synapse 已有：
- `RouterSupervisor` (multiagent/router.rs) — LLM-driven 路由，sub-agent 作为 tool
- `AgentTool` (multiagent/agent_tool.rs) — `Arc<dyn Agent>` → `InvokableTool` 桥接
- `DeepAgent` (adk/deep.rs) — 旧版"任务分解"实现，但 sub_agents 字段**根本没用**

### Router vs Orchestrator 区别

| 维度 | Router/Supervisor | Host/Orchestrator |
|------|-------------------|-------------------|
| 任务结构 | 单任务 → 选合适 agent | 复合任务 → 分解为多 sub-tasks |
| 调用方式 | 顺序，可能多轮 | **并行 dispatch**（一次 response 多个 tool_calls） |
| 角色定位 | "调度员"（接电话） | "项目经理"（拆活 → 派发 → 汇总） |
| 结果处理 | 看到结果就回答 | 显式 **synthesize** 步骤 |

### 目标

实现 **Host/Orchestrator** MultiAgent 模式：
- LLM 显式分解任务为独立 sub-tasks
- 一次 response 内**并行 dispatch** 多个 sub-agents
- 收集所有 sub-agent 结果，**synthesize** 成最终答案

## 设计

### 核心洞察

**Orchestrator = RouterSupervisor + 不同的 system prompt**

LLM 都支持 parallel tool calls（OpenAI, Anthropic, Neuron）。只要 prompt 引导 LLM：
- "decompose task"
- "dispatch in parallel"
- "synthesize results"

就能实现 orchestrator 行为，**无需新的运行循环**。

### 架构

```
┌────────────────────────────────────────────────────────────┐
│  Runner                                                    │
│   │                                                        │
│   ▼                                                        │
│  OrchestratorAgent (Host)                                  │
│   │  - system prompt: "decompose → parallel → synthesize"  │
│   │  - 内部 = RouterSupervisor<M> (复用)                   │
│   │  - tools: [AgentTool("researcher"), AgentTool("writer")│
│   │            AgentTool("analyst")]                        │
│   ▼                                                        │
│  ReAct Loop (single iteration, parallel tool calls)        │
│   │  LLM Call #1:                                          │
│   │    response.tool_calls = [                             │
│   │      (route_to_researcher, "find X"),                  │
│   │      (route_to_analyst, "compute Y"),                 │
│   │      (route_to_writer, "draft Z"),                     │
│   │    ]  ← ALL IN PARALLEL                                │
│   │                                                        │
│   │  System executes all 3 AgentTool.invoke() in parallel  │
│   │  (tokio::join! or sequential await)                    │
│   │                                                        │
│   │  LLM Call #2: synthesize all 3 results                 │
│   │    response: text("Final answer: ... X, Y, Z ...")     │
│   ▼                                                        │
│  Final AgentEvent (synthesized)                            │
└────────────────────────────────────────────────────────────┘
```

### 组件

#### OrchestratorAgent

```rust
// src/adk/multiagent/orchestrator.rs
pub struct OrchestratorAgent<M: BaseChatModel + 'static> {
    inner: RouterSupervisor<M>,
    name: String,
}

impl<M: BaseChatModel + 'static> OrchestratorAgent<M> {
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> OrchestratorAgentBuilder<M> {
        OrchestratorAgentBuilder::new(name, model)
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for OrchestratorAgent<M> {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.inner.description() }
    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        self.inner.run(input).await
    }
    async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct OrchestratorAgentBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    agents: Vec<Arc<dyn Agent>>,
    max_iterations: usize,
}

impl<M: BaseChatModel + 'static> OrchestratorAgentBuilder<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: "Orchestrator agent".to_string(),
            model,
            agents: Vec::new(),
            max_iterations: 12,
        }
    }
    pub fn description(mut self, d: impl Into<String>) -> Self { ... }
    pub fn add_agent(mut self, agent: Arc<dyn Agent>) -> Self { ... }
    pub fn max_iterations(mut self, n: usize) -> Self { ... }
    pub fn build(self) -> OrchestratorAgent<M> {
        // 1. Build agent descriptions
        // 2. Substitute into orchestrator prompt template
        // 3. Build inner RouterSupervisor with custom system_prompt
        // 4. Return OrchestratorAgent
    }
}
```

#### Orchestrator System Prompt

```markdown
# src/adk/multiagent/orchestrator_system.md

You are an orchestrator agent that decomposes complex tasks into independent
sub-tasks and dispatches them to specialized agents in parallel.

Available agents (each is a tool):
{agent_descriptions}

When given a task:
1. Analyze the task and decide if it can be decomposed into independent sub-tasks
2. For each sub-task, identify the most appropriate agent
3. Call the appropriate agent tools in PARALLEL — make multiple tool calls in a
   SINGLE response to dispatch all sub-tasks at once
4. When all sub-agents have responded, synthesize their results into a coherent
   final answer

You may also call agents sequentially if a sub-task depends on another's
output. Pass relevant context in the "task" parameter.

When you have synthesized all sub-agent results, provide the final answer
WITHOUT any tool calls.
```

**关键设计**：
- "PARALLEL" + "SINGLE response" 明确提示 LLM 一次 dispatch 多个
- 允许 sequential（依赖场景）
- 与 RouterSupervisor 的 prompt 关键差异：强调分解 + 并行 dispatch

### 文件结构

```
src/adk/multiagent/
├── mod.rs                       (添加 OrchestratorAgent re-export)
├── agent_tool.rs                (复用)
├── router.rs                    (复用)
├── orchestrator.rs              (NEW)
└── orchestrator_system.md       (NEW)

tests/orchestrator_test.rs       (NEW — 5 integration tests)
```

### 公开 API

```rust
// src/adk/multiagent/mod.rs
pub use orchestrator::{OrchestratorAgent, OrchestratorAgentBuilder};

// 用户使用
let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .max_iterations(12)
    .build();

let result = orchestrator.run(input).await?;
// LLM 会自动：分解 → 并行 dispatch → synthesize
```

### 测试

#### 测试矩阵

| # | 测试名 | 场景 | 验证点 |
|---|--------|------|--------|
| 1 | `test_orchestrator_decomposes_and_dispatches` | 复合任务 → 1 sub-agent | 1 个 sub-agent 被调用；最终 answer 是合成结果 |
| 2 | `test_orchestrator_parallel_dispatch` | 复合任务 → 2+ sub-agents 一次 dispatch | **同一 mock response 多个 tool_calls**；2 个 sub-agents 都收到 task |
| 3 | `test_orchestrator_synthesizes_results` | 2 sub-agents 返回 → synthesis | 最终 answer 包含两个 sub-agent 的输出 |
| 4 | `test_orchestrator_handles_subagent_failure` | 1 sub-agent 失败 | orchestrator 不报错；最终 answer 反映部分失败 |
| 5 | `test_orchestrator_nested` | Orchestrator 嵌套 | 外层 → 内层 → sub-agent；调用链正确 |

#### Mock 关键差异

与 RouterSupervisor 的测试关键差异：**Test 2 用 parallel tool calls**

```rust
// Router test: 一次一个 tool_call
vec![
    tool_call_message("route_to_a", "..."),  // 1 call
    Message::assistant("..."),                // final
]

// Orchestrator test: 一次多个 tool_calls
vec![
    Message {
        tool_calls: Some(vec![
            ToolCall { name: "route_to_a", ... },
            ToolCall { name: "route_to_b", ... },
        ]),
    },
    Message::assistant("synthesized"),
]
```

这测试 mock 模型能正确返回 parallel tool calls（验证 LLM 输出格式 + orchestrator 框架能并行处理）。

### 与 RouterSupervisor 的对比

| 维度 | RouterSupervisor | OrchestratorAgent |
|------|------------------|-------------------|
| 内部 | `ChatModelAgent<M, AgentTool>` | `RouterSupervisor<M>` (wrap) |
| Prompt 重点 | "选合适 agent" | "分解 + 并行 dispatch + 合成" |
| 默认 max_iterations | 8 | 12（更多 sub-tasks 需要更多迭代） |
| Sub-agent 数量 | 适合 1-3 | 适合 2-10（更复杂场景） |
| LLM 行为 | 倾向串行（看结果再决定） | 倾向并行（一次 dispatch） |
| 适用场景 | 客服、单一任务分类 | 研究、写作、多视角分析 |

### 关键设计决策

1. **复用 RouterSupervisor 框架** — 0 新运行循环
2. **不同 system prompt** — 引导 LLM 行为差异
3. **包装而非继承** — `OrchestratorAgent { inner: RouterSupervisor }` 简单清晰
4. **Builder API 镜像 RouterSupervisor** — 用户学习成本低
5. **Document 增量** — 强调"decompose + parallel + synthesize" 三个动作

### 实施分解

| Step | 内容 | 估时 |
|------|------|------|
| 1 | 写 orchestrator.rs + system_prompt + Builder | 30min |
| 2 | 加 mod.rs re-export | 5min |
| 3 | 5 个 integration tests (含 parallel tool_calls mock) | 45min |
| 4 | 跑测试 + 修 edge cases | 15min |
| 5 | commit + merge | 10min |
| **总计** | | **~2h** |

## 后续工作（不在本 spec 范围）

- **Conversational Handoff**（agent 间 transfer 对话）
- **混合 Router + Orchestrator**（按任务类型动态选择）
- **Sub-task 显式 plan JSON**（更结构化的分解）
- **并行执行可视化**（debug 工具）
