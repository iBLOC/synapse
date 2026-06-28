# Synapse MultiAgent (Router/Supervisor Pattern) 设计 Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-28
> **关联项目：** 神经突触 Synapse（本项目）/ 神经元 Neuron / 前额叶 PFC

## 背景与目标

### 现状

Synapse 已有以下 Agent 抽象：
- `Agent` trait (`src/adk/agent/mod.rs`) — 统一 Agent 接口
- `ChatModelAgent<M, T>` (`src/adk/react.rs`) — 单 agent + 工具调用 + ReAct loop
- `GraphReActAgent` (`src/adk/agent/react/graph_react.rs`) — Graph-based ReAct agent
- `DeepAgent<M>` (`src/adk/deep.rs`) — 任务分解为子任务（不是路由）
- `Supervisor<M>` (`src/adk/deep.rs`) — **雏形**：用 LLM 选择 agent 名，自由文本输出

### 问题

`Supervisor<M>` 已有但很弱：
1. 路由完全靠 LLM 自由文本输出（"respond with only the agent name"），LLM 经常输出多余解释或拼错名字
2. 单轮路由：选一次，调用一次，没有"调多个 agent 后汇总"的能力
3. 没有嵌套支持
4. 没有 max_iterations 防护（`Supervisor::run` 直接调 `agent.run(input)`，如果子 agent 是 ReAct agent 才间接防护）

### 目标

实现**Tool-based Router/Supervisor**：
- Supervisor 把 sub-agent 包装为 tool，自身跑 ReAct loop
- 复用 `ChatModelAgent<M, T>` 的 ReAct 引擎（不写新循环）
- LLM 通过 tool call 路由（结构化，可靠）
- 支持多轮：可调多个 sub-agent，最后汇总
- 支持嵌套：sub-agent 可以是另一个 RouterSupervisor
- 错误处理：sub-agent 失败时返回 tool result JSON，让 LLM 决定下一步

## 设计

### 核心洞察

**不需要写新的运行循环**。RouterSupervisor 就是一个普通的 `ChatModelAgent<M, AgentTool>`，只是它的 tools 列表里是 `AgentTool`（包装其他 Agent）而不是普通工具。

### 架构

```
┌─────────────────────────────────────────────────────────────┐
│  Runner                                                     │
│   │                                                         │
│   ▼                                                         │
│  RouterSupervisor (Host Agent)                              │
│   │  - system prompt: "你是 supervisor, 协调以下 agents..." │
│   │  - tools: [AgentTool("researcher"),                     │
│   │           AgentTool("coder"),                           │
│   │           AgentTool("writer")]                          │
│   │  - 复用 ChatModelAgent<M, AgentTool> 的 ReAct loop       │
│   ▼                                                         │
│  ReAct Loop (multi-turn)                                    │
│   │  iteration 1: think → tool_call(agent="researcher")     │
│   │  iteration 2: think → tool_call(agent="coder")          │
│   │  iteration 3: think → "汇总完成" → final answer         │
│   ▼                                                         │
│  AgentTool.invoke(args)                                     │
│   │  1. 解析 args → AgentInput                              │
│   │  2. 调用 sub-agent.run()                                 │
│   │  3. 提取 sub-agent output → JSON 返回                   │
│   ▼                                                         │
│  SubAgent (任意实现 Agent trait)                            │
│   │  - ChatModelAgent                                       │
│   │  - GraphReActAgent                                      │
│   │  - DeepAgent                                            │
│   │  - 另一个 RouterSupervisor (nested)                     │
└─────────────────────────────────────────────────────────────┘
```

### 组件

#### AgentTool — Agent ↔ Tool 桥接

**目的**：把任意 `Arc<dyn Agent>` 包装为 `InvokableTool`，让 Supervisor 的 LLM 可以"调用"sub-agent。

```rust
// src/adk/multiagent/agent_tool.rs
pub struct AgentTool {
    agent: Arc<dyn Agent>,
    info: ToolInfo,  // 缓存：避免每次 invoke 重复生成 schema
}

impl AgentTool {
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        let info = ToolInfo {
            name: format!("route_to_{}", agent.name()),
            description: agent.description(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The task to delegate to this agent"
                    }
                },
                "required": ["task"]
            }),
        };
        Self { agent, info }
    }
}

impl BaseTool for AgentTool {
    fn info(&self) -> impl Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move { Ok(self.info.clone()) }
    }
}

#[async_trait]
impl InvokableTool for AgentTool {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        // 1. parse args
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let task = args["task"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("missing 'task' field".into()))?;

        // 2. build AgentInput
        let input = AgentInput {
            messages: vec![Message::user(task)],
            enable_streaming: false,
        };

        // 3. run sub-agent (errors encoded in result JSON, not thrown)
        let event = match self.agent.run(input).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(json!({
                    "agent": self.agent.name(),
                    "success": false,
                    "error": e.to_string(),
                }).to_string());
            }
        };

        // 4. extract text output
        let text = event.output.message
            .as_ref()
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        // 5. return as JSON
        Ok(json!({
            "agent": self.agent.name(),
            "success": event.error.is_none(),
            "output": text,
        }).to_string())
    }
}
```

**关键设计点**：
- `info` 缓存：避免每次 invoke 重新生成 JSON schema
- tool name 加 `route_to_` 前缀：避免和普通工具冲突，明确语义
- `info()` 第一次 `async move` 捕获 `self.info.clone()`：因为 `self` 不能 move 进 `async move`
- **错误编码为 JSON 而非抛错**：让 LLM 看到错误后可决策（不导致整个 run 失败）

#### RouterSupervisor — 顶层入口

```rust
// src/adk/multiagent/router.rs
pub struct RouterSupervisor<M: BaseChatModel + 'static> {
    inner: ChatModelAgent<M, AgentTool>,
    name: String,
}

impl<M: BaseChatModel + 'static> RouterSupervisor<M> {
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> RouterSupervisorBuilder<M> {
        RouterSupervisorBuilder::new(name, model)
    }
}

impl<M: BaseChatModel + Send + Sync + 'static> Agent for RouterSupervisor<M> {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.inner.description() }
    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        self.inner.run(input).await
    }
    async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct RouterSupervisorBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<AgentTool>>,
    max_iterations: usize,
    system_prompt: Option<String>,
}

impl<M: BaseChatModel + 'static> RouterSupervisorBuilder<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: "Router/Supervisor agent".to_string(),
            model,
            tools: Vec::new(),
            max_iterations: 8,
            system_prompt: None,
        }
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn add_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.tools.push(Arc::new(AgentTool::new(agent)));
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn system_prompt(mut self, p: impl Into<String>) -> Self {
        self.system_prompt = Some(p.into());
        self
    }

    pub fn build(self) -> RouterSupervisor<M> {
        let inner = ChatModelAgent::new(
            self.name.clone(),
            self.description.clone(),
            self.model,
        )
        .with_tools(self.tools)
        .max_iterations(self.max_iterations);

        // 如果提供了 system_prompt，prepend 到第一条 message
        // 实际实现：使用一个内嵌的 system message 注入
        // (ChatModelAgent 当前不支持 system prompt，扩展它或直接在 run 前注入)

        RouterSupervisor {
            inner,
            name: self.name,
        }
    }
}
```

**类型决策**：
- `RouterSupervisor<M>` 内部是 `ChatModelAgent<M, AgentTool>` — 具体 T = AgentTool
- 用户构造时不需要指定 T
- 嵌套：把 `Arc<RouterSupervisor<M>>` 包装为 `Arc<dyn Agent>`（自动），再喂给外层 RouterSupervisor

**实现注意**：`ChatModelAgent` 当前不支持 system prompt 注入。扩展方案：
- **方案 A**（推荐）：扩展 `ChatModelAgent` 加 `with_system_prompt`
- **方案 B**：在 `RouterSupervisor` 自己的 `run()` 里 prepend system message 到 input.messages

选 A：和 `with_tools` 等 builder 方法保持一致。

### 数据流

#### System Prompt

**默认 supervisor system prompt** (`supervisor_system.md`)：

```markdown
You are a supervisor that coordinates specialized agents to answer user questions.

Available agents (each is a tool):
{agent_descriptions}

When the user asks a question:
1. Decide which agent is best suited to handle the task
2. Call that agent by invoking the appropriate tool with a clear "task" description
3. If the result is incomplete or another agent's expertise is needed, call another agent
4. When you have enough information, provide a final synthesized answer (without tool calls)

You may call multiple agents in sequence. Each agent sees only the task you give it, not the full conversation history. So pass relevant context in the "task" parameter.
```

`{agent_descriptions}` 在 build 时填充（每个 Agent 的 `name` + `description`）。

#### 一次完整 run 的数据流

```
[User input]
   "What is the capital of France and what's 2+2?"
   │
   ▼
[Supervisor LLM Call #1]
   system: "You are a supervisor..."
   messages: [user("What is the capital of France and what's 2+2?")]
   tools: [route_to_researcher, route_to_calculator]
   │
   │ → response: tool_calls=[(route_to_researcher, "capital of France")]
   │
   ▼
[Tool Execution: AgentTool("researcher").invoke]
   args.task = "What is the capital of France?"
   │
   │ → ResearcherAgent.run()
   │   → uses OpenAI: "The capital of France is Paris."
   │
   ▼
[Tool result: "{\"agent\":\"researcher\",\"output\":\"The capital of France is Paris.\"}"]
   │
   ▼
[Supervisor LLM Call #2]
   messages: [user, assistant(tool_call), tool(result="Paris...")]
   │
   │ → response: tool_calls=[(route_to_calculator, "2+2")]
   │
   ▼
[Tool Execution: AgentTool("calculator").invoke]
   → "2+2 equals 4"
   │
   ▼
[Tool result: "{\"agent\":\"calculator\",\"output\":\"2+2 equals 4\"}"]
   │
   ▼
[Supervisor LLM Call #3]
   messages: [user, asst+tool, asst+tool]
   │
   │ → response: text("The capital of France is Paris, and 2+2 equals 4.")
   │   (NO tool calls → loop exits)
   │
   ▼
[Final AgentEvent]
   output.message = AssistantMessage("The capital of France is Paris, and 2+2 equals 4.")
   action = None
```

**关键点**：
- Supervisor 自己就是 LLM agent，复用 `ChatModelAgent::run_loop`
- Sub-agent 完全无状态：每次调用从 0 开始 messages（只接收 task）
- 父 agent 的 messages 历史完整保留在 supervisor 内部
- Sub-agent 自己的"工具调用"对父透明（父只看到子返回的最终文本）

#### 消息格式

Sub-agent 返回给 supervisor 的 tool result 格式（JSON string）：
```json
{
  "agent": "researcher",
  "success": true,
  "output": "The capital of France is Paris."
}
```

或失败时：
```json
{
  "agent": "researcher",
  "success": false,
  "error": "tool error: ..."
}
```

这样 supervisor 后续多轮决策时，能看到结构化的"researcher 说..."信息。

### 错误处理

#### 错误场景与策略

| 场景 | 处理策略 |
|------|---------|
| Sub-agent 超时 | 返回错误 JSON 给 supervisor，让 LLM 决定重试/换 agent |
| Sub-agent 内部 panic | 捕获 panic（如有），返回 error JSON |
| LLM 选了不存在的 tool | 框架层防御：只有 registered 的 agent 才会出现在 tools 里 |
| LLM 路由到错的 agent | 由 LLM 看到 sub-agent 返回结果后自行决定是否换 agent |
| Sub-agent 任务完成不完整 | LLM 看到不完整结果后可继续调另一个 agent |
| RouterSupervisor 自身 max_iterations 耗尽 | 返回 `AgentError::MaxIterationsExceeded` |
| 嵌套时内层 max_iterations 耗尽 | 编码到 JSON，传递给外层 supervisor |

#### 错误信息作为 tool result 返回

**关键设计**：`AgentTool::invoke` **永远返回 `Ok(String)`**，错误信息编码在 JSON 里。

```rust
async fn invoke(&self, arguments: String) -> ToolResult_ {
    // 即使子 agent 失败，也返回 Ok(JSON)
    match self.try_invoke(arguments).await {
        Ok(text) => Ok(json!({
            "agent": self.agent.name(),
            "success": true,
            "output": text,
        }).to_string()),
        Err(e) => Ok(json!({
            "agent": self.agent.name(),
            "success": false,
            "error": e.to_string(),
        }).to_string()),
    }
}
```

**理由**：supervisor 是 LLM agent，它需要看到错误信息才能智能地决定下一步。如果 `invoke` 抛 `ToolError`，会触发框架级错误处理（导致整个 run 失败），supervisor 没机会 fallback。

**唯一例外**：`arguments` 解析失败（`serde_json::from_str` 失败）→ 返回 `ToolError::InvalidArguments`。这是真正的"调用错误"，不是 sub-agent 业务错误，框架层处理更合理。

#### max_iterations 防死循环

```rust
RouterSupervisor::builder("supervisor", model)
    .max_iterations(8)  // 8 次足够：2-3 个 sub-agent + 总结
    .add_agent(researcher)
    .add_agent(calculator)
    .build()
```

`ChatModelAgent::run_loop` 已经有 `max_iterations` 限制，RouterSupervisor 直接透传。

#### System Prompt 引导"放弃"决策

默认 prompt 中说明：
```
If an agent returns an error, you may:
1. Retry with the same agent (if the error seems transient)
2. Try a different agent (if the error is permanent)
3. Give up and inform the user
```

（实际不强制写在 prompt 里 — 由 LLM 自行判断；上述是 reference behavior）

### 文件结构

```
src/adk/multiagent/
├── mod.rs                # 公共 API 导出
├── agent_tool.rs         # AgentTool: Agent ↔ Tool 桥接
├── router.rs             # RouterSupervisor + Builder
└── supervisor_system.md  # 默认 system prompt (embed_str)
```

修改文件：
- `src/adk/mod.rs` — 添加 `pub mod multiagent;` 和 re-export
- `src/adk/react.rs` — `ChatModelAgent` 加 `with_system_prompt` builder 方法
- `src/adk/deep.rs` — 保留 `Supervisor`（不删，向后兼容）；加 doc 链接到 `multiagent::RouterSupervisor`

### 公开 API

```rust
// src/adk/multiagent/mod.rs
pub use agent_tool::AgentTool;
pub use router::{RouterSupervisor, RouterSupervisorBuilder};

// 用户使用
let supervisor = RouterSupervisor::builder("supervisor", model)
    .description("Coordinates research and writing")
    .add_agent(Arc::new(researcher_agent) as Arc<dyn Agent>)
    .add_agent(Arc::new(writer_agent))
    .max_iterations(8)
    .build();

let result = supervisor.run(input).await?;
```

### 测试

#### 测试矩阵

| # | 测试名 | 场景 | 验证点 |
|---|--------|------|--------|
| 1 | `test_router_routes_to_single_agent` | Supervisor 路由到 1 个 sub-agent | sub-agent 收到正确 task；最终输出包含 sub-agent 的输出 |
| 2 | `test_router_routes_to_multiple_agents` | Supervisor 路由到 2 个 sub-agent | tool result 都被 supervisor 看到；最终 answer 包含两个 agent 的信息 |
| 3 | `test_router_no_route_needed` | 简单问题不需要 sub-agent | supervisor 直接返回，不调任何 agent |
| 4 | `test_router_handles_agent_failure` | Sub-agent 内部失败 | supervisor 看到 success=false；可以继续或放弃 |
| 5 | `test_router_max_iterations_exceeded` | 死循环场景 | 第 N 次迭代后返回 MaxIterationsExceeded |
| 6 | `test_router_nested` | RouterSupervisor 嵌套 | 外层 → 内层 → sub-agent；嵌套调用链正确 |
| 7 | `test_router_empty_agents` | 没有任何 sub-agent 注册 | supervisor 只能用纯 LLM 能力回答（不报错） |

#### Mock 工具

复用 `tests/graph_react_agent_test.rs` 里的 `MockChatModel` 模式：
- `MockChatModel`: 实现 `BaseChatModel`，按调用次数返回预设 `Message`（text 或带 tool_calls）
- `MockAgent`: 简单实现 `Agent` trait，记录 `received_tasks`，返回预设 text
- `FailingMockAgent`: `run` 永远返回 `AgentError::Agent("mock failure")`

#### 测试结构

文件：`tests/router_supervisor_test.rs`

```rust
struct MockAgent {
    name: String,
    description: String,
    response: String,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Agent for MockAgent {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.description.clone() }
    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let task = input.messages.last()
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.received_tasks.lock().unwrap().push(task);
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant(self.response.clone())),
                action: None,
            },
            error: None,
        })
    }
    async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> AgentResult<AgentEvent> {
        unimplemented!()
    }
}

struct FailingMockAgent { name: String, description: String }
#[async_trait]
impl Agent for FailingMockAgent {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.description.clone() }
    async fn run(&self, _: AgentInput) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("mock failure".to_string()))
    }
    async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> AgentResult<AgentEvent> {
        unimplemented!()
    }
}

// 7 个测试函数...
```

### 与现有代码的整合

| 现有 | 关系 | 说明 |
|------|------|------|
| `ChatModelAgent<M, T>` (src/adk/react.rs) | **复用 + 微扩展** | RouterSupervisor 内部用 = `ChatModelAgent<M, AgentTool>`；扩展加 `with_system_prompt` |
| `GraphReActAgent` (src/adk/agent/react/) | **可选** | 也可作为 RouterSupervisor 内部引擎（如果用户指定），但 ChatModelAgent 够用 |
| `Supervisor<M>` (src/adk/deep.rs) | **共存** | 旧的自由 LLM 路由保留，向后兼容；doc 指向新的 RouterSupervisor |
| `DeepAgent<M>` (src/adk/deep.rs) | **共存** | 任务分解模式 ≠ 路由模式 |
| `Agent` trait | **复用** | RouterSupervisor 本身就是 Agent，可以嵌套 |
| `BaseTool`, `InvokableTool` | **复用** | AgentTool 实现这俩 trait |
| `MockChatModel` (test helper) | **复用** | router 测试直接用现有模式 |

### 风险与权衡

| 风险 | 影响 | 缓解 |
|------|------|------|
| LLM tool call 支持 | RouterSupervisor 依赖 LLM tool calling | 所有主流 LLM (OpenAI/Anthropic/Neuron) 都支持；测试用 mock 不依赖外部 |
| Tool call 命名冲突 | 用户的普通工具和 AgentTool 同名 | AgentTool 加 `route_to_` 前缀避免冲突 |
| Sub-agent 性能 | 多层嵌套时延迟累积 | 默认 max_iterations=8 限制总调用次数 |
| Sub-agent 状态丢失 | sub-agent 每次从头开始 | 通过 `task` 参数显式传上下文；如需状态共享，可加 StatefulAgent 包装（后续工作） |

### 实施分解

| Step | 内容 | 估时 |
|------|------|------|
| 1 | 扩展 `ChatModelAgent::with_system_prompt` | 15min |
| 2 | 实现 `AgentTool` + unit test | 30min |
| 3 | 实现 `RouterSupervisor` + Builder | 30min |
| 4 | 写 system prompt template (`supervisor_system.md`) | 10min |
| 5 | `src/adk/multiagent/mod.rs` 公共 API | 10min |
| 6 | `src/adk/mod.rs` 集成 + 旧 Supervisor doc 链接 | 10min |
| 7 | 7 个 integration tests | 1h |
| 8 | 跑全套测试 + 修 edge cases | 30min |
| 9 | commit + merge | 10min |
| **总计** | | **~3h** |

## 后续工作（不在本 spec 范围）

- `Host/Orchestrator` 模式（DeepAgent 升级版，多 agent 协同任务分解）
- `Conversational Handoff` 模式（agent 间移交对话）
- `StatefulAgent` 包装（让 sub-agent 跨调用保持状态）
- `RouterStrategy` 抽象（除 LLM 外支持 keyword/rule-based 路由）
