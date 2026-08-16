# Phase 3: Graph-Based ReAct Agent Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan.

**Goal:** 实现基于 Graph 基础设施的完整 ReAct Agent，对标 Eino `flow/agent/react/react.go`。核心是基于 `CompiledGraph` 构建 ReAct Agent，支持状态管理、流式输出。

**Architecture:**
- 创建 `GraphReActAgent` 结构体，内部持有 `CompiledGraph`
- 状态通过 Context 在节点间传递（使用 JSON 序列化/反序列化）
- ChatModelNode → Conditional(ToolCall?) → ToolsNode / END
- ToolsNode → Conditional(ReturnDirectly?) → DirectReturn / ChatModelNode
- 支持 `generate`（非流式）和 `stream`（流式）

**Tech Stack:** Rust, tokio, async_trait

---

## Chunk 1: 状态管理与 State Handler（~100行）

**Files:**
- Create: `src/adk/agent/react/state.rs`
- Modify: `src/adk/agent/react/mod.rs`

- [ ] **Step 1: 创建 `src/adk/agent/react/` 目录和 `mod.rs`**

```rust
pub mod state;
pub mod graph_react;
```

- [ ] **Step 2: 创建 `src/adk/agent/react/state.rs` — ReAct State**

```rust
use crate::schema::Message;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReActState {
    pub messages: Vec<Message>,
    pub return_directly_tool_call_id: Option<String>,
}

impl ReActState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            return_directly_tool_call_id: None,
        }
    }
}
```

- [ ] **Step 3: 创建 State PreHandler 函数**

```rust
/// PreHandler for ChatModel node: appends input messages to state
pub fn chat_model_pre_handler(
    ctx: &Context,
    input: &[Message],
    state: &mut ReActState,
) -> Result<Vec<Message>, String> {
    state.messages.extend(input.clone());
    Ok(state.messages.clone())
}

/// PreHandler for Tools node: appends tool result message to state
pub fn tools_node_pre_handler(
    ctx: &Context,
    input: &Message,
    state: &mut ReActState,
) -> Result<Message, String> {
    state.messages.push(input.clone());
    if let Some(tool_calls) = &input.tool_calls {
        // Check if any tool should return directly
        for tc in tool_calls {
            // Return directly if configured (omitted for now)
        }
    }
    Ok(input.clone())
}
```

- [ ] **Step 4: 运行 `cargo build` 确认 Chunk 1 编译通过**

---

## Chunk 2: GraphReActAgent 核心结构（~150行）

**Files:**
- Modify: `src/adk/agent/react/mod.rs`
- Modify: `src/adk/agent/react/graph_react.rs`

- [ ] **Step 1: 创建 `src/adk/agent/react/graph_react.rs`**

```rust
use std::sync::Arc;
use crate::schema::Message;
use crate::compose::{Graph, CompiledGraph, START, END};
use crate::compose::runnable::{Runnable, RunnableFuture, RunnableResult, RunnableError, Context};
use crate::adk::agent::{Agent, AgentInput, AgentEvent, AgentError, AgentOutput};

pub struct GraphReActAgent {
    name: String,
    description: String,
    compiled_graph: Arc<CompiledGraph<Vec<Message>, Message>>,
    max_steps: usize,
}
```

- [ ] **Step 2: 实现 `Agent` trait for `GraphReActAgent`**

```rust
#[async_trait]
impl Agent for GraphReActAgent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let compiled_graph = self.compiled_graph.clone();
        let max_steps = self.max_steps;
        
        let output = compiled_graph
            .invoke(Context::new(), input.messages)
            .await
            .map_err(|e| AgentError::Agent(e.to_string()))?;
            
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(output),
                action: None,
            },
            error: None,
        })
    }

    async fn resume(&self, _checkpoint: Vec<u8>, _info: ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}
```

- [ ] **Step 3: 实现 Builder pattern for GraphReActAgent**

```rust
impl GraphReActAgent {
    pub fn builder() -> GraphReActAgentBuilder {
        GraphReActAgentBuilder::new()
    }
}

pub struct GraphReActAgentBuilder {
    name: String,
    description: String,
    model: Arc<dyn crate::components::model::BaseChatModel>,
    tools: Vec<Arc<dyn crate::components::tool::BaseTool>>,
    max_steps: usize,
}

impl GraphReActAgentBuilder {
    pub fn new() -> Self {
        Self {
            name: "ReActAgent".to_string(),
            description: "".to_string(),
            model: ...,
            tools: Vec::new(),
            max_steps: 12,
        }
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn crate::components::tool::BaseTool>>) -> Self { ... }
    pub fn max_steps(mut self, max: usize) -> Self { ... }
    pub fn build(self) -> Result<GraphReActAgent, AgentError> { ... }
}
```

- [ ] **Step 4: 运行 `cargo build` 确认 Chunk 2 编译通过**

---

## Chunk 3: Graph 构建逻辑（~200行）

**Files:**
- Modify: `src/adk/agent/react/graph_react.rs`

- [ ] **Step 1: 实现 `build_graph` 函数**

```rust
fn build_graph(
    model: Arc<dyn crate::components::model::BaseChatModel>,
    tools_node: Arc<dyn Runnable<Message, Vec<Message>>>,
) -> Result<CompiledGraph<Vec<Message>, Message>, AgentError> {
    let mut graph = Graph::<Vec<Message>, Message>::new(NodeTriggerMode::AnyPredecessor);
    
    // Add ChatModel node
    let chat_model_node = ChatModelNode::new(model);
    graph.add_node("chat", Arc::new(chat_model_node));
    
    // Add Tools node  
    graph.add_node("tools", tools_node);
    
    // Add Lambda nodes for branching
    // Conditional: if has_tool_calls -> tools, else -> END
    // Tools Conditional: if return_directly -> direct_return, else -> chat
    
    // Edges
    graph.add_edge(START, "chat")?;
    graph.add_edge("chat", "tools")?; // via branch
    graph.add_edge("chat", END)?;   // via branch
    graph.add_edge("tools", "chat")?;
    graph.add_edge("tools", END)?;
    graph.add_edge("direct_return", END)?;
    
    graph.compile_with_options(GraphCompileOptions {
        max_steps: 12,
        ..Default::default()
    })
}
```

- [ ] **Step 2: 创建 ChatModelNode Runnable**

```rust
struct ChatModelNode {
    model: Arc<dyn crate::components::model::BaseChatModel>,
}

impl ChatModelNode {
    fn new(model: Arc<dyn crate::components::model::BaseChatModel>) -> Self {
        Self { model }
    }
}

impl Runnable<Vec<Message>, Message> for ChatModelNode {
    fn invoke(&self, ctx: Context, input: Vec<Message>) -> RunnableFuture<Message> {
        Box::pin(async move {
            let options = GenerateOptions {
                tools: Some(get_tool_infos(&self.model)), // Omitted
                ..Default::default()
            };
            self.model
                .generate(input, options)
                .await
                .map_err(|e| RunnableError::Invoke(e.to_string()))
        })
    }
}
```

- [ ] **Step 3: 实现条件分支逻辑**

```rust
// Branch condition: check if message has tool_calls
fn has_tool_calls(msg: &Message) -> Option<String> {
    if msg.tool_calls.is_some() && !msg.tool_calls.as_ref().unwrap().is_empty() {
        Some("tools".to_string())
    } else {
        None // Go to END
    }
}
```

- [ ] **Step 4: 运行 `cargo build` 确认 Chunk 3 编译通过**

---

## Chunk 4: 集成测试（~100行）

**Files:**
- Create: `tests/graph_react_agent_test.rs`

- [ ] **Step 1: 写一个简单的 mock model 和 tool**

```rust
struct MockChatModel { ... }

#[async_trait::async_trait]
impl BaseChatModel for MockChatModel {
    async fn generate(&self, messages: Vec<Message>, opts: GenerateOptions) -> Result<Message, ModelError> {
        // Return a message with a tool call
        Message {
            role: RoleType::Assistant,
            content: Content::Text("calling tool".to_string()),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "my_tool".to_string(),
                    arguments: "{}".to_string(),
                },
            }]),
            ..Default::default()
        }
    }
}
```

- [ ] **Step 2: 测试 agent 生成**

```rust
#[tokio::test]
async fn test_graph_react_agent_single_turn() {
    let model = Arc::new(MockChatModel::new());
    let tool = TestTool::new("my_tool", "result");
    
    let agent = GraphReActAgent::builder()
        .with_model(model)
        .with_tools(vec![tool])
        .build()
        .unwrap();
        
    let input = AgentInput {
        messages: vec![Message::user("hello")],
        enable_streaming: false,
    };
    
    let event = agent.run(input).await.unwrap();
    assert!(event.output.message.is_some());
}
```

- [ ] **Step 3: 测试多轮对话**

- [ ] **Step 4: 运行 `cargo test` 确认全部通过**

---

## Chunk 5: 完善与清理

**Files:**
- Modify: `src/adk/agent/react/mod.rs` (exports)
- Modify: `src/adk/mod.rs`

- [ ] **Step 1: 更新 `src/adk/agent/react/mod.rs` 导出**

```rust
pub mod state;
pub mod graph_react;

pub use graph_react::{GraphReActAgent, GraphReActAgentBuilder};
```

- [ ] **Step 2: 在 `src/adk/mod.rs` 添加导出**

```rust
pub mod agent;
pub use agent::{Agent, AgentInput, AgentEvent, ...};
pub use agent::react::{GraphReActAgent, GraphReActAgentBuilder};
```

- [ ] **Step 3: 运行完整 `cargo test`**

- [ ] **Step 4: 修复所有诊断错误**

- [ ] **Step 5: git commit**
