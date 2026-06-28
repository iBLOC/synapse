# Synapse MultiAgent (Router/Supervisor) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Tool-based Router/Supervisor MultiAgent 模式，让 supervisor 通过 LLM tool calling 路由到多个 sub-agent，复用现有 `ChatModelAgent` ReAct loop。

**Architecture:** 引入 `AgentTool`（把 `Arc<dyn Agent>` 包装为 `InvokableTool`）。`RouterSupervisor<M>` 内部就是 `ChatModelAgent<M, AgentTool>`，tools 列表里全是 `AgentTool`。LLM 通过 tool call 选择要调用的 sub-agent，sub-agent 的结果编码为 JSON 返回给 supervisor。多轮路由自然由 ReAct loop 支持。

**Tech Stack:** Rust, tokio, async-trait, serde, serde_json, synapML crate (synapse)

**Spec Reference:** `docs/superpowers/specs/2026-06-28-multiagent-router-design.md`

---

## File Structure

**New Files:**
- `src/adk/multiagent/mod.rs` — 公共 API 导出
- `src/adk/multiagent/agent_tool.rs` — AgentTool: Agent ↔ Tool 桥接
- `src/adk/multiagent/router.rs` — RouterSupervisor + RouterSupervisorBuilder
- `src/adk/multiagent/supervisor_system.md` — 默认 system prompt 模板
- `tests/router_supervisor_test.rs` — 7 个 integration tests

**Modified Files:**
- `src/adk/mod.rs` — 添加 `pub mod multiagent;` + re-exports
- `src/adk/react.rs` — `ChatModelAgent` 加 `with_system_prompt` builder 方法
- `src/adk/deep.rs` — 保留旧 Supervisor，加 doc 链接到新 RouterSupervisor

---

## Chunk 1: AgentTool 基础（最小可工作单元）

**目标：** 创建 `AgentTool` 及其 5 个 unit tests，独立验证 AgentTool 的所有行为。这是后续 RouterSupervisor 的基石。

### Task 1.1: 创建 multiagent 模块骨架

**Files:**
- Create: `src/adk/multiagent/mod.rs`

- [ ] **Step 1: 创建模块文件**

```rust
// src/adk/multiagent/mod.rs
//! MultiAgent patterns: Router/Supervisor coordination.
//!
//! This module provides:
//! - [`AgentTool`]: Bridges `Agent` trait to `InvokableTool` so any agent
//!   can be exposed as a tool that a supervisor LLM can call.
//! - [`RouterSupervisor`]: A supervisor agent that uses an LLM to route
//!   tasks to specialized sub-agents, with multi-turn support.
//!
//! See `docs/superpowers/specs/2026-06-28-multiagent-router-design.md` for design.

pub mod agent_tool;
pub mod router;

pub use agent_tool::AgentTool;
pub use router::{RouterSupervisor, RouterSupervisorBuilder};
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | tail -5`
Expected: warning about unused submodules but no errors (submodules don't exist yet)

- [ ] **Step 3: 暂时创建空的子模块文件**

```rust
// src/adk/multiagent/agent_tool.rs
// placeholder, will be filled in Task 1.2
```

```rust
// src/adk/multiagent/router.rs
// placeholder, will be filled in Task 3.x
```

- [ ] **Step 4: 编译验证**

Run: `cargo build 2>&1 | tail -5`
Expected: success, no warnings

- [ ] **Step 5: 提交**

```bash
git add src/adk/multiagent/
git commit -m "feat(multiagent): scaffold multiagent module"
```

### Task 1.2: AgentTool 字段定义 + 第一个测试

**Files:**
- Modify: `src/adk/multiagent/agent_tool.rs`
- Create: `tests/agent_tool_test.rs`（unit test 文件）

- [ ] **Step 1: 写测试 — AgentTool::new 生成正确 ToolInfo**

```rust
// tests/agent_tool_test.rs
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo};
use synapse::adk::multiagent::AgentTool;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use synapse::schema::{Content, Message};

struct MockAgent {
    name: String,
    description: String,
    response: String,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

impl MockAgent {
    fn new(name: &str, description: &str, response: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            description: description.to_string(),
            response: response.to_string(),
            received_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        let task = input
            .messages
            .last()
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
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_agent_tool_info_has_route_to_prefix() {
    let agent = MockAgent::new("researcher", "Researches topics", "answer");
    let tool = AgentTool::new(agent.clone());

    let info = tool.info().await.unwrap();
    assert_eq!(info.name, "route_to_researcher");
    assert_eq!(info.description, "Researches topics");
}

#[tokio::test]
async fn test_agent_tool_info_has_task_parameter() {
    let agent = MockAgent::new("coder", "Writes code", "code");
    let tool = AgentTool::new(agent);

    let info = tool.info().await.unwrap();
    let params = info.parameters.as_object().expect("params should be object");
    assert_eq!(params.get("type").and_then(|v| v.as_str()), Some("object"));
    let properties = params
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("properties");
    assert!(properties.contains_key("task"));
    let required = params
        .get("required")
        .and_then(|v| v.as_array())
        .expect("required");
    assert!(required.iter().any(|v| v.as_str() == Some("task")));
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test agent_tool_test 2>&1 | tail -10`
Expected: FAIL — `AgentTool` not found, or `info()` not found

- [ ] **Step 3: 实现 AgentTool struct + ::new + BaseTool::info**

```rust
// src/adk/multiagent/agent_tool.rs
//! Bridge between Agent trait and InvokableTool.
//!
//! Wraps any `Arc<dyn Agent>` as a `Tool` so a supervisor LLM can route to
//! sub-agents via tool calls (more reliable than free-form text routing).

use std::future::Future;
use std::sync::Arc;

use crate::adk::agent::{Agent, AgentInput};
use crate::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use crate::schema::{Content, Message, ToolInfo};

/// Wraps an `Agent` as an `InvokableTool` for supervisor LLM routing.
pub struct AgentTool {
    agent: Arc<dyn Agent>,
    cached_info: ToolInfo,
}

impl AgentTool {
    /// Create an `AgentTool` from any `Arc<dyn Agent>`.
    /// The tool name is `route_to_<agent_name>`, and it accepts a single
    /// `task: string` argument passed to the sub-agent as a user message.
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        let cached_info = ToolInfo {
            name: format!("route_to_{}", agent.name()),
            description: agent.description(),
            parameters: serde_json::json!({
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
        Self { agent, cached_info }
    }

    /// Returns the agent name (without the `route_to_` prefix).
    pub fn agent_name(&self) -> String {
        self.agent.name()
    }
}

impl BaseTool for AgentTool {
    fn info(&self) -> impl Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move { Ok(self.cached_info.clone()) }
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

Run: `cargo test --test agent_tool_test 2>&1 | tail -10`
Expected: PASS — 2 tests

- [ ] **Step 5: 提交**

```bash
git add src/adk/multiagent/agent_tool.rs tests/agent_tool_test.rs
git commit -m "feat(multiagent): AgentTool with cached ToolInfo"
```

### Task 1.3: AgentTool::invoke 成功路径

**Files:**
- Modify: `src/adk/multiagent/agent_tool.rs`
- Modify: `tests/agent_tool_test.rs`

- [ ] **Step 1: 添加测试 — invoke 传递 task 给 agent 并返回 JSON 格式输出**

Add to `tests/agent_tool_test.rs`:

```rust
#[tokio::test]
async fn test_agent_tool_invoke_passes_task_and_returns_json() {
    let agent = MockAgent::new("researcher", "Researches topics", "Paris is the capital");
    let tool = AgentTool::new(agent.clone());

    let result = tool
        .invoke(r#"{"task": "What is the capital of France?"}"#.to_string())
        .await
        .unwrap();

    // Verify task was passed
    let received = agent.received_tasks.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "What is the capital of France?");

    // Verify result is JSON with expected fields
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["agent"], "researcher");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["output"], "Paris is the capital");
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test agent_tool_test test_agent_tool_invoke 2>&1 | tail -10`
Expected: FAIL — `invoke` method not implemented

- [ ] **Step 3: 实现 `InvokableTool for AgentTool`（成功路径）**

Add to `src/adk/multiagent/agent_tool.rs`:

```rust
use async_trait::async_trait;

#[async_trait]
impl InvokableTool for AgentTool {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        // 1. parse args
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(format!("parse arguments: {}", e)))?;
        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("missing 'task' field".into()))?;

        // 2. build AgentInput
        let input = AgentInput {
            messages: vec![Message::user(task.to_string())],
            enable_streaming: false,
        };

        // 3. run sub-agent (errors encoded in result JSON, not thrown)
        let event = match self.agent.run(input).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(serde_json::json!({
                    "agent": self.agent.name(),
                    "success": false,
                    "error": e.to_string(),
                })
                .to_string());
            }
        };

        // 4. extract text output
        let text = event
            .output
            .message
            .as_ref()
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        // 5. return as JSON
        Ok(serde_json::json!({
            "agent": self.agent.name(),
            "success": event.error.is_none(),
            "output": text,
        })
        .to_string())
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

Run: `cargo test --test agent_tool_test 2>&1 | tail -10`
Expected: PASS — 3 tests

- [ ] **Step 5: 提交**

```bash
git add src/adk/multiagent/agent_tool.rs tests/agent_tool_test.rs
git commit -m "feat(multiagent): AgentTool::invoke with success path + JSON output"
```

### Task 1.4: AgentTool::invoke 失败路径

**Files:**
- Modify: `tests/agent_tool_test.rs`

- [ ] **Step 1: 添加测试 — 当 sub-agent 失败时，返回 success=false JSON 而非抛错**

Add to `tests/agent_tool_test.rs`:

```rust
struct FailingMockAgent {
    name: String,
    description: String,
}

#[async_trait]
impl Agent for FailingMockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, AgentError> {
        Err(AgentError::Agent("mock failure".to_string()))
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_agent_tool_invoke_returns_error_as_json() {
    let agent: Arc<dyn Agent> = Arc::new(FailingMockAgent {
        name: "broken".to_string(),
        description: "Always fails".to_string(),
    });
    let tool = AgentTool::new(agent);

    // Should NOT throw, should return Ok with success=false JSON
    let result = tool
        .invoke(r#"{"task": "anything"}"#.to_string())
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["agent"], "broken");
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"]
        .as_str()
        .unwrap()
        .contains("mock failure"));
}
```

- [ ] **Step 2: 运行测试验证通过（实现已经支持）**

Run: `cargo test --test agent_tool_test 2>&1 | tail -10`
Expected: PASS — 4 tests (the implementation from Task 1.3 already handles this)

- [ ] **Step 3: 添加测试 — 解析错误时抛 ToolError**

Add to `tests/agent_tool_test.rs`:

```rust
#[tokio::test]
async fn test_agent_tool_invoke_throws_on_invalid_args() {
    let agent = MockAgent::new("x", "x", "x");
    let tool = AgentTool::new(agent);

    // Malformed JSON
    let result = tool.invoke("not json".to_string()).await;
    assert!(matches!(
        result,
        Err(ToolError::InvalidArguments(_))
    ));

    // Missing 'task' field
    let result = tool.invoke(r#"{}"#.to_string()).await;
    assert!(matches!(
        result,
        Err(ToolError::InvalidArguments(_))
    ));
}
```

- [ ] **Step 4: 运行测试验证通过**

Run: `cargo test --test agent_tool_test 2>&1 | tail -10`
Expected: PASS — 5 tests

- [ ] **Step 5: 提交**

```bash
git add tests/agent_tool_test.rs
git commit -m "test(multiagent): AgentTool failure and invalid args tests"
```

### Task 1.5: 验证 Chunk 1 完整

- [ ] **Step 1: 跑全套测试确认无回归**

Run: `cargo test 2>&1 | grep "test result" | tail -5`
Expected: 之前的所有测试都通过，AgentTool 新增 5 个测试通过

- [ ] **Step 2: 提交（如有未提交改动）**

```bash
git status
# If anything uncommitted:
git add -A
git commit -m "chore: chunk 1 final cleanup"
```

---

## Chunk 2: 扩展 ChatModelAgent + RouterSupervisor 基础

**目标：** 扩展 `ChatModelAgent` 加 `with_system_prompt`，实现 `RouterSupervisor` + Builder，4 个 unit tests。

### Task 2.1: 扩展 ChatModelAgent::with_system_prompt

**Files:**
- Modify: `src/adk/react.rs`

- [ ] **Step 1: 阅读现有的 ChatModelAgent**

Read `src/adk/react.rs` line 53-101，确认 ChatModelAgent 的字段。

- [ ] **Step 2: 添加 system_prompt 字段和 builder 方法**

Modify `src/adk/react.rs`:

在 `ChatModelAgent` struct (line 53) 添加字段：
```rust
pub struct ChatModelAgent<M: BaseChatModel, T: InvokableTool> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<T>>,
    skills: Arc<tokio::sync::RwLock<SkillManager>>,
    max_iterations: usize,
    temperature: Option<f32>,
    tool_middleware: Vec<Arc<dyn ToolMiddleware>>,
    system_prompt: Option<String>,  // NEW
}
```

在 `ChatModelAgent::new` (line 65-76) 添加字段：
```rust
pub fn new(name: impl Into<String>, description: impl Into<String>, model: Arc<M>) -> Self {
    Self {
        name: name.into(),
        description: description.into(),
        model,
        tools: Vec::new(),
        skills: Arc::new(tokio::sync::RwLock::new(SkillManager::new())),
        max_iterations: 10,
        temperature: None,
        tool_middleware: Vec::new(),
        system_prompt: None,  // NEW
    }
}
```

在 `with_tool_middleware` 方法后添加：
```rust
pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
    self.system_prompt = Some(prompt.into());
    self
}
```

- [ ] **Step 3: 修改 `run_loop` 注入 system prompt**

In `ChatModelAgent::run_loop` (line 157), modify the first line of the function to inject the system message:

```rust
pub async fn run_loop(&self, messages: Vec<Message>) -> Result<AgentEvent, AgentError> {
    let mut iterations = 0;
    let mut current_messages = messages;

    // NEW: Prepend system prompt if configured
    if let Some(prompt) = &self.system_prompt {
        current_messages.insert(0, Message::system(prompt.clone()));
    }

    // ... rest of function unchanged
```

- [ ] **Step 4: 编译并运行现有测试**

Run: `cargo test 2>&1 | tail -5`
Expected: 全部通过

- [ ] **Step 5: 提交**

```bash
git add src/adk/react.rs
git commit -m "feat(react): ChatModelAgent with_system_prompt builder"
```

### Task 2.2: RouterSupervisor 基础 + Builder

**Files:**
- Modify: `src/adk/multiagent/router.rs`
- Create: `tests/router_supervisor_unit_test.rs`

- [ ] **Step 1: 写测试 — RouterSupervisor 构造后具有正确的 name/description**

```rust
// tests/router_supervisor_unit_test.rs
use std::sync::Arc;
use synapse::adk::agent::Agent;
use synapse::adk::multiagent::{RouterSupervisor, AgentTool};
use synapse::components::model::BaseChatModel; // need a mock

// We'll use a stub model — see step 2 for implementation
struct StubModel;
#[async_trait::async_trait]
impl BaseChatModel for StubModel {
    async fn generate(
        &self,
        _: Vec<synapse::schema::Message>,
        _: synapse::components::model::GenerateOptions,
    ) -> synapse::components::model::ModelResult<synapse::schema::Message> {
        Ok(synapse::schema::Message::assistant("ok"))
    }
    async fn stream(
        &self,
        _: Vec<synapse::schema::Message>,
        _: synapse::components::model::GenerateOptions,
    ) -> synapse::components::model::ModelResult<synapse::components::model::StreamReader> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_router_supervisor_builder_sets_name_and_description() {
    use std::sync::Arc;
    use async_trait::async_trait;
    use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo};

    struct NopAgent;
    #[async_trait]
    impl Agent for NopAgent {
        fn name(&self) -> String { "x".into() }
        fn description(&self) -> String { "x".into() }
        async fn run(&self, _: AgentInput) -> Result<AgentEvent, AgentError> {
            unimplemented!()
        }
        async fn resume(&self, _: Vec<u8>, _: ResumeInfo) -> Result<AgentEvent, AgentError> {
            unimplemented!()
        }
    }

    let model = Arc::new(StubModel);
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .description("Coordinates work")
        .add_agent(Arc::new(NopAgent) as Arc<dyn Agent>)
        .build();

    assert_eq!(supervisor.name(), "supervisor");
    assert_eq!(supervisor.description(), "Coordinates work");
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test router_supervisor_unit_test 2>&1 | tail -10`
Expected: FAIL — `RouterSupervisor` not found

- [ ] **Step 3: 实现 RouterSupervisor + Builder**

```rust
// src/adk/multiagent/router.rs
//! Router/Supervisor pattern: an LLM-powered host that delegates to sub-agents
//! via tool calling.
//!
//! See `docs/superpowers/specs/2026-06-28-multiagent-router-design.md` for the
//! full design rationale.

use std::sync::Arc;

use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo};
use crate::adk::multiagent::agent_tool::AgentTool;
use crate::adk::react::ChatModelAgent;
use crate::components::model::BaseChatModel;

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("supervisor_system.md");

/// A supervisor agent that uses an LLM to route tasks to specialized
/// sub-agents. The sub-agents are exposed as tools (with names like
/// `route_to_<agent_name>`), and the LLM decides which to call.
///
/// Internally a `ChatModelAgent<M, AgentTool>` — no new run loop is
/// introduced; we reuse the existing ReAct loop.
pub struct RouterSupervisor<M: BaseChatModel + 'static> {
    inner: ChatModelAgent<M, AgentTool>,
    name: String,
    description: String,
}

impl<M: BaseChatModel + 'static> RouterSupervisor<M> {
    /// Create a new builder for a `RouterSupervisor`.
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> RouterSupervisorBuilder<M> {
        RouterSupervisorBuilder::new(name, model)
    }
}

#[async_trait::async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for RouterSupervisor<M> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        self.inner.run(input).await
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

/// Builder for `RouterSupervisor`.
pub struct RouterSupervisorBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<AgentTool>>,
    max_iterations: usize,
    system_prompt: Option<String>,
}

impl<M: BaseChatModel + 'static> RouterSupervisorBuilder<M> {
    /// Create a new builder with default values.
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

    /// Add a sub-agent that the supervisor can route to.
    pub fn add_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.tools.push(Arc::new(AgentTool::new(agent)));
        self
    }

    /// Set the maximum number of LLM iterations (default 8).
    /// Each iteration is one LLM call + tool execution.
    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    /// Override the default system prompt.
    pub fn system_prompt(mut self, p: impl Into<String>) -> Self {
        self.system_prompt = Some(p.into());
        self
    }

    /// Build the `RouterSupervisor`.
    pub fn build(self) -> RouterSupervisor<M> {
        let default_prompt = DEFAULT_SYSTEM_PROMPT.to_string();
        let agent_descriptions: Vec<String> = self
            .tools
            .iter()
            .map(|t| format!("- {}: {}", t.agent_name(), "<see tool description>"))
            .collect();
        let prompt = self
            .system_prompt
            .unwrap_or_else(|| default_prompt.replace("{agent_descriptions}", &agent_descriptions.join("\n")));

        let inner = ChatModelAgent::new(
            self.name.clone(),
            self.description.clone(),
            self.model,
        )
        .with_tools(self.tools)
        .max_iterations(self.max_iterations)
        .with_system_prompt(prompt);

        RouterSupervisor {
            inner,
            name: self.name,
            description: self.description,
        }
    }
}
```

- [ ] **Step 4: 创建默认 system prompt 文件**

Create `src/adk/multiagent/supervisor_system.md`:

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

- [ ] **Step 5: 运行测试验证通过**

Run: `cargo test --test router_supervisor_unit_test 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add src/adk/multiagent/router.rs src/adk/multiagent/supervisor_system.md tests/router_supervisor_unit_test.rs
git commit -m "feat(multiagent): RouterSupervisor + Builder"
```

### Task 2.3: 验证 ChatModelAgent 现有测试无回归

- [ ] **Step 1: 跑全部测试**

Run: `cargo test 2>&1 | tail -10`
Expected: 全部通过，ReactAgent 测试、ChatModelAgent 测试都通过

- [ ] **Step 2: 如果有回归，定位并修复**

检查 ChatModelAgent 的现有测试是否因 system_prompt 注入而失败。如有，添加 system_prompt 为 None 时的兼容性测试。

---

## Chunk 3: 公共 API + 集成到 adk::mod

**目标：** 把 multiagent 模块加到公共 API，保留旧 Supervisor（向后兼容）但加 doc 链接。

### Task 3.1: 导出 multiagent 到 adk::mod

**Files:**
- Modify: `src/adk/mod.rs`

- [ ] **Step 1: 读取并修改 src/adk/mod.rs**

Read current `src/adk/mod.rs` (should be 8 lines). Modify to:

```rust
pub mod agent;
pub mod deep;
pub mod multiagent;
pub mod react;
pub mod runner;
pub mod state;

pub use agent::react::{GraphReActAgent, GraphReActAgentBuilder};
pub use agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput};
pub use multiagent::{AgentTool, RouterSupervisor, RouterSupervisorBuilder};
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | tail -5`
Expected: success

- [ ] **Step 3: 提交**

```bash
git add src/adk/mod.rs
git commit -m "feat(adk): export multiagent module"
```

### Task 3.2: 给旧 Supervisor 加 doc 链接

**Files:**
- Modify: `src/adk/deep.rs`

- [ ] **Step 1: 在 Supervisor struct 上加 doc 注释**

In `src/adk/deep.rs`, find `pub struct Supervisor<M: BaseChatModel>` (around line 156). Add a doc comment:

```rust
/// **Note:** This is a basic LLM-based router. For new code, prefer
/// [`crate::adk::multiagent::RouterSupervisor`], which uses structured
/// tool calls for reliable multi-turn routing and supports nesting.
pub struct Supervisor<M: BaseChatModel> {
```

- [ ] **Step 2: 编译验证**

Run: `cargo build 2>&1 | tail -5`
Expected: success

- [ ] **Step 3: 提交**

```bash
git add src/adk/deep.rs
git commit -m "docs(deep): add doc link from Supervisor to RouterSupervisor"
```

### Task 3.3: 验证 Chunk 3 完整

- [ ] **Step 1: 跑全部测试**

Run: `cargo test 2>&1 | tail -5`
Expected: 全部通过

---

## Chunk 4: 7 个 integration tests

**目标：** 实现 7 个 router integration tests，覆盖所有关键场景。

### Task 4.1: 测试基础设施 + 第一个测试 (routes_to_single_agent)

**Files:**
- Create: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 创建测试文件 + MockChatModel helper + test 1**

```rust
// tests/router_supervisor_test.rs
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo};
use synapse::adk::multiagent::RouterSupervisor;
use synapse::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use synapse::components::model::StreamReader;
use synapse::schema::{Content, FunctionCall, Message, RoleType, ToolCall};

// --- MockChatModel ---
struct MockChatModel {
    responses: Vec<Message>,
    index: AtomicUsize,
}

impl MockChatModel {
    fn new(responses: Vec<Message>) -> Arc<Self> {
        Arc::new(Self {
            responses,
            index: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl BaseChatModel for MockChatModel {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<Message> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        if self.responses.is_empty() {
            return Err(ModelError::Api("No responses configured".to_string()));
        }
        let i = idx % self.responses.len();
        Ok(self.responses[i].clone())
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<StreamReader> {
        Err(ModelError::Api("Streaming not implemented".to_string()))
    }
}

// --- MockAgent ---
struct MockAgent {
    name: String,
    description: String,
    response: String,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

impl MockAgent {
    fn new(name: &str, description: &str, response: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            description: description.to_string(),
            response: response.to_string(),
            received_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        let task = input
            .messages
            .last()
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
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

// --- FailingMockAgent ---
struct FailingMockAgent {
    name: String,
    description: String,
}

#[async_trait]
impl Agent for FailingMockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, AgentError> {
        Err(AgentError::Agent("mock failure".to_string()))
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

// Helper: create a tool_call message
fn tool_call_message(name: &str, args: &str) -> Message {
    Message {
        role: RoleType::Assistant,
        content: Content::Text("".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![ToolCall {
            id: format!("call_{}", name),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: args.to_string(),
            },
        }]),
    }
}

#[tokio::test]
async fn test_router_routes_to_single_agent() {
    let model = MockChatModel::new(vec![
        tool_call_message(
            "route_to_researcher",
            r#"{"task": "capital of France?"}"#,
        ),
        Message::assistant("The answer is Paris."),
    ]);

    let agent = MockAgent::new("researcher", "Researches topics", "Paris is the capital");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(agent.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("What is the capital of France?")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    // Verify sub-agent received the task
    let received = agent.received_tasks.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "capital of France?");

    // Verify final output contains sub-agent's text
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Paris"), "got: {}", s);
    } else {
        panic!("expected text content");
    }
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --test router_supervisor_test test_router_routes_to_single_agent 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 如果失败，调试并修复**

Possible issues: 实际 ChatModelAgent 的 run_loop 行为可能与 mock 假设不同（如对 tool_call 的处理细节）。修复 mock 或 RouterSupervisor 行为。

- [ ] **Step 4: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_routes_to_single_agent"
```

### Task 4.2: 测试 2 — routes_to_multiple_agents

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 2**

Add to `tests/router_supervisor_test.rs`:

```rust
#[tokio::test]
async fn test_router_routes_to_multiple_agents() {
    let model = MockChatModel::new(vec![
        tool_call_message("route_to_researcher", r#"{"task": "Q1"}"#),
        tool_call_message("route_to_calc", r#"{"task": "Q2"}"#),
        Message::assistant("Both done."),
    ]);

    let researcher = MockAgent::new("researcher", "Researches", "R answer");
    let calc = MockAgent::new("calc", "Calculates", "C answer");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(researcher.clone() as Arc<dyn Agent>)
        .add_agent(calc.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Q1 and Q2")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    assert_eq!(researcher.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(calc.received_tasks.lock().unwrap().len(), 1);
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Both done"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_routes_to_multiple_agents 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_routes_to_multiple_agents"
```

### Task 4.3: 测试 3 — no_route_needed

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 3**

```rust
#[tokio::test]
async fn test_router_no_route_needed() {
    let model = MockChatModel::new(vec![Message::assistant("Direct answer, no agent needed")]);
    let agent = MockAgent::new("a", "A", "A");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(agent.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Hi")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    assert_eq!(agent.received_tasks.lock().unwrap().len(), 0);
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert_eq!(s, "Direct answer, no agent needed");
    } else {
        panic!("expected text");
    }
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_no_route_needed 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_no_route_needed"
```

### Task 4.4: 测试 4 — handles_agent_failure

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 4**

```rust
#[tokio::test]
async fn test_router_handles_agent_failure() {
    let model = MockChatModel::new(vec![
        tool_call_message("route_to_broken", r#"{"task": "X"}"#),
        Message::assistant("Could not complete"),
    ]);

    let failing: Arc<dyn Agent> = Arc::new(FailingMockAgent {
        name: "broken".to_string(),
        description: "Always fails".to_string(),
    });
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(failing)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Try X")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    // The supervisor should still return a result (not error out)
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Could not complete"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_handles_agent_failure 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_handles_agent_failure"
```

### Task 4.5: 测试 5 — max_iterations_exceeded

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 5**

```rust
#[tokio::test]
async fn test_router_max_iterations_exceeded() {
    // Model always returns tool_call, never text → loop until max
    let model = MockChatModel::new(vec![tool_call_message(
        "route_to_a",
        r#"{"task": "loop"}"#,
    )]);

    let agent = MockAgent::new("a", "A", "A");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .max_iterations(3)
        .add_agent(agent as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Loop")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await;
    assert!(matches!(result, Err(AgentError::MaxIterationsExceeded)));
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_max_iterations_exceeded 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_max_iterations_exceeded"
```

### Task 4.6: 测试 6 — nested

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 6**

```rust
#[tokio::test]
async fn test_router_nested() {
    // Inner model: ① tool_call to sub, ② text result
    let inner_model = MockChatModel::new(vec![
        tool_call_message("route_to_sub", r#"{"task": "inner task"}"#),
        Message::assistant("Inner result"),
    ]);

    // Outer model: ① tool_call to inner, ② text synthesized
    let outer_model = MockChatModel::new(vec![
        tool_call_message("route_to_inner", r#"{"task": "outer task"}"#),
        Message::assistant("Outer synthesized: Inner result"),
    ]);

    let sub = MockAgent::new("sub", "Sub agent", "Sub result");
    let inner_supervisor = RouterSupervisor::builder("inner", inner_model)
        .add_agent(sub.clone() as Arc<dyn Agent>)
        .build();

    let outer_supervisor = RouterSupervisor::builder("outer", outer_model)
        .add_agent(Arc::new(inner_supervisor) as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Nested test")],
        enable_streaming: false,
    };
    let result = outer_supervisor.run(input).await.expect("run failed");

    // Verify sub agent was called
    assert_eq!(sub.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(sub.received_tasks.lock().unwrap()[0], "inner task");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Inner result"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_nested 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_nested"
```

### Task 4.7: 测试 7 — empty_agents

**Files:**
- Modify: `tests/router_supervisor_test.rs`

- [ ] **Step 1: 添加测试 7**

```rust
#[tokio::test]
async fn test_router_empty_agents() {
    let model = MockChatModel::new(vec![Message::assistant("Pure LLM answer")]);
    let supervisor = RouterSupervisor::builder("supervisor", model).build();

    let input = AgentInput {
        messages: vec![Message::user("Hi")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert_eq!(s, "Pure LLM answer");
    } else {
        panic!("expected text");
    }
}
```

- [ ] **Step 2: 运行并验证**

Run: `cargo test --test router_supervisor_test test_router_empty_agents 2>&1 | tail -10`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add tests/router_supervisor_test.rs
git commit -m "test(multiagent): router_empty_agents"
```

---

## Chunk 5: 验证 + 合并

**目标：** 全套测试通过，合并到 master。

### Task 5.1: 跑全部测试套件

- [ ] **Step 1: 跑所有测试**

Run: `cargo test 2>&1 | tail -20`
Expected: 所有测试通过（包括之前 87 + 新增 5 unit + 7 integration = 99 tests）

- [ ] **Step 2: 修复任何回归**

如果有测试失败：
1. 定位是 RouterSupervisor 的 bug 还是 mock 假设错误
2. 修复
3. 重跑

- [ ] **Step 3: 跑 clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -20`
Expected: 没有 warnings

- [ ] **Step 4: 跑 cargo fmt**

Run: `cargo fmt --check 2>&1 | tail -5`
Expected: no diff. 如果有 diff：`cargo fmt`

- [ ] **Step 5: 最终 commit**

```bash
git status
# 修复 clippy/fmt 引入的改动
git add -A
git commit -m "style: cargo fmt + clippy fixes"
```

### Task 5.2: 更新文档

**Files:**
- Modify: `README.md` (if exists)

- [ ] **Step 1: 在 README 添加 MultiAgent 章节**

如果项目有 README.md，添加简短示例：
```rust
// Multi-agent: router with sub-agents
let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .build();
let result = supervisor.run(input).await?;
```

- [ ] **Step 2: 提交**

```bash
git add README.md
git commit -m "docs: MultiAgent usage example in README"
```

### Task 5.3: 合并到 master

- [ ] **Step 1: 切换到 master 并 merge**

```bash
git checkout master
git merge feature/multiagent --no-ff -m "Merge feature/multiagent: MultiAgent Router/Supervisor pattern

Tool-based Router/Supervisor:
- AgentTool bridges Agent trait to InvokableTool
- RouterSupervisor uses ChatModelAgent's ReAct loop (no new run loop)
- LLM routes via tool calls (structured, reliable)
- Supports multi-turn routing and nested supervisors
- Sub-agent errors encoded in tool result JSON (LLM can recover)

12 new tests (5 AgentTool + 7 RouterSupervisor integration).
99 total tests (no regression)."
```

- [ ] **Step 2: 最终验证**

Run: `cargo test 2>&1 | tail -5`
Expected: 99 tests pass

- [ ] **Step 3: 清理 worktree**

```bash
git worktree remove .worktrees/multiagent
git branch -d feature/multiagent
```

---

## 风险与回退

| 风险 | 缓解 |
|------|------|
| ChatModelAgent 加 system_prompt 破坏现有行为 | Chunk 2.1 加完后立即跑全部测试验证 |
| Mock 模型序列与实际 ChatModelAgent 行为不匹配 | 第一个测试 (Task 4.1) 立即验证，失败则先调试 |
| Mock 模型不支持 with_tools 为空 (test 7) | 如有问题，调整 ChatModelAgent 处理空 tools |
| 嵌套 RouterSupervisor 编译失败（generic bound） | 如有问题，加更多 Send + Sync 约束 |

## 完成标准

- [ ] AgentTool 5 个 unit tests pass
- [ ] RouterSupervisor builder test pass
- [ ] 7 个 integration tests pass
- [ ] 全部 99 个测试 pass (87 existing + 12 new)
- [ ] cargo clippy clean
- [ ] cargo fmt clean
- [ ] 合并到 master
