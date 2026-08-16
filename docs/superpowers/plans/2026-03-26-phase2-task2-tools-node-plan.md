# ToolsNode Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Synapse 的 `ToolsNode`，与 Eino `compose/tool_node.go` 功能拉齐。核心是让 `ToolsNode` 实现 `Runnable<Message, Vec<Message>>`，能够从 `Message.tool_calls` 解析工具调用，并发或顺序执行工具，返回 `ToolMessage` 列表。

**Architecture:** 
- `ToolsNode` 是 `Runnable<Message, Vec<Message>>`，其 `invoke`/`stream` 方法签名与 Eino 对齐
- 内部构建 `toolsTuple` 索引所有工具，按 `tool_call.function.name` 查表
- 支持 middleware chain（中间件模式），支持 parallel / sequential 执行
- 支持 `UnknownToolsHandler`（LLM 幻觉工具时的兜底）和 `ToolArgumentsHandler`（参数预处理）
- 所有 4 种工具接口都支持：`InvokableTool`, `StreamableTool`, `EnhancedInvokableTool`, `EnhancedStreamableTool`
- Synapse 已有完整的 `components/tool/mod.rs`（工具接口 + StreamReader），只需构建 node 层

**Tech Stack:** Rust, async_trait, tokio, futures, serde_json

---

## Chunk 1: 数据结构 & Builder（~150行）

**Files:**
- Create: `src/compose/tool_node/mod.rs`
- Modify: `src/compose/mod.rs`, `src/components/tool/mod.rs`

- [ ] **Step 1: 创建 `src/compose/tool_node/mod.rs`，写入基础结构和错误类型**

```rust
use thiserror::Error;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Error)]
pub enum ToolsNodeError {
    #[error("tool {0} not found in toolsNode")]
    ToolNotFound(String),
    #[error("invalid input: expected Assistant role, got {0:?}")]
    InvalidRole(crate::schema::RoleType),
    #[error("no tool calls found in input message")]
    NoToolCalls,
    #[error("tool {0} is not invokable, streamable, enhanced invokable or enhanced streamable")]
    NotInvokable(String),
    #[error("tool execution error [{name}]: {0}")]
    Execution { name: String, #[source] error: String },
    #[error("failed to execute tool arguments handler for {name}: {0}")]
    ArgumentsHandler { name: String, #[source] error: String },
}

pub struct ToolInput {
    pub name: String,
    pub arguments: String,
    pub call_id: String,
}

pub struct ToolOutput {
    pub result: String,
}

pub struct StreamToolOutput {
    pub result: crate::compose::StreamReader<String>,
}

pub struct EnhancedInvokableToolOutput {
    pub result: crate::schema::ToolResult,
}

pub struct EnhancedStreamToolOutput {
    pub result: crate::compose::StreamReader<crate::schema::ToolResult>,
}
```

- [ ] **Step 2: 添加 `toolsTuple` 结构和 `conv_tools` 转换函数**

```rust
struct ToolsTuple {
    indexes: HashMap<String, usize>,
    endpoints: Vec<Option<Arc<dyn Send + Sync>>,  // InvokableToolEndpoint
    stream_endpoints: Vec<Option<Arc<dyn Send + Sync>>>, // StreamableToolEndpoint
    enhanced_invokable_endpoints: Vec<Option<Arc<dyn Send + Sync>>>,
    enhanced_stream_endpoints: Vec<Option<Arc<dyn Send + Sync>>>,
}

fn conv_tools(tools: &[Arc<dyn crate::components::tool::BaseTool>])
    -> Result<ToolsTuple, ToolsNodeError>
{
    // 对每个 tool:
    // 1. 调用 info() 获取 name
    // 2. 尝试向下转型为 InvokableTool / StreamableTool / EnhancedInvokableTool / EnhancedStreamableTool
    // 3. 如果都不是，返回 ToolsNodeError::NotInvokable
    // 4. 构建 endpoints 向量，索引 map
}
```

- [ ] **Step 3: 创建 `ToolsNodeConfig` 和 `ToolsNodeBuilder`**

```rust
pub struct ToolsNodeConfig {
    pub tools: Vec<Arc<dyn crate::components::tool::BaseTool>>,
    pub unknown_tool_handler: Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync>>,
    pub execute_sequentially: bool,
    pub tool_arguments_handler: Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync>>,
    pub tool_call_middlewares: Vec<ToolMiddleware>,
}

pub struct ToolMiddleware {
    pub invokable: Option<Arc<dyn Fn(Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>) -> Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static> + Send + Sync>>,
    pub streamable: Option<...>,
    pub enhanced_invokable: Option<...>,
    pub enhanced_streamable: Option<...>,
}

pub struct ToolsNode {
    tuple: ToolsTuple,
    unknown_tool_handler: Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync>>,
    execute_sequentially: bool,
    tool_arguments_handler: Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync>>,
}

impl ToolsNode {
    pub fn new(config: ToolsNodeConfig) -> Result<Self, ToolsNodeError> { ... }
}
```

- [ ] **Step 4: 更新 `src/compose/mod.rs` 添加 `pub mod tool_node`**

- [ ] **Step 5: 运行 `cargo build` 确认 Chunk 1 编译通过**

---

## Chunk 2: `Runnable` 实现 — `invoke` & `stream`（~200行）

**Files:**
- Modify: `src/compose/tool_node/mod.rs`

- [ ] **Step 1: 为 `ToolsNode` 实现 `Runnable<Message, Vec<Message>>`**

```rust
use crate::compose::runnable::{Runnable, RunnableFuture, RunnableResult, Context, RunnableError};

impl Runnable<crate::schema::Message, Vec<crate::schema::Message>> for ToolsNode {
    fn invoke(&self, ctx: Context, input: crate::schema::Message) -> RunnableFuture<Vec<crate::schema::Message>> {
        Box::pin(async move {
            // 1. 验证 role == Assistant
            // 2. 检查 tool_calls 是否为空
            // 3. 构建 tool_call_tasks（从 input.tool_calls 解析）
            // 4. 如果 execute_sequentially，逐个执行；否则并发
            // 5. 收集结果，构建 ToolMessage Vec
            // 6. Ok(messages)
        })
    }

    fn stream(&self, ctx: Context, input: crate::schema::Message) -> RunnableFuture<crate::compose::StreamReader<Vec<crate::schema::Message>>> {
        Box::pin(async move {
            // 类似 invoke，但每个工具调用返回 StreamReader
            // 最后 merge 所有 stream readers
        })
    }

    fn name(&self) -> String {
        "tools_node".to_string()
    }
}
```

- [ ] **Step 2: 实现 `gen_tool_call_tasks` — 解析 `Message.tool_calls` → `toolCallTask` 列表**

```rust
struct ToolCallTask {
    name: String,
    call_id: String,
    arg: String,
    executed: bool,
    output: Option<String>,
    stream_output: Option<crate::compose::StreamReader<String>>,
    enhanced_output: Option<crate::schema::ToolResult>,
    enhanced_stream_output: Option<crate::compose::StreamReader<crate::schema::ToolResult>>,
    err: Option<String>,
    endpoint: Option<...>,
    stream_endpoint: Option<...>,
    enhanced_endpoint: Option<...>,
    enhanced_stream_endpoint: Option<...>,
    use_enhanced: bool,
}

fn gen_tool_call_tasks(&self, input: &crate::schema::Message) -> Result<Vec<ToolCallTask>, ToolsNodeError>
{
    // 1. 验证 input.role == RoleType::Assistant
    // 2. 遍历 input.tool_calls
    // 3. 用 name 查 self.tuple.indexes
    // 4. 如果找不到，调用 unknown_tool_handler 或返回错误
    // 5. 如果找到了，设置对应的 endpoint
    // 6. 如果有 tool_arguments_handler，调用它处理参数
}
```

- [ ] **Step 3: 实现并发/顺序执行逻辑**

```rust
fn execute_tasks(&self, tasks: &mut [ToolCallTask], ctx: &Context) {
    if self.execute_sequentially {
        for task in tasks.iter_mut() {
            self.run_task(task, ctx);
        }
    } else {
        // 使用 tokio::spawn 并发执行
        let handles: Vec<_> = tasks.iter_mut().map(|t| {
            let task = t;
            let ctx = ctx.clone();
            tokio::spawn(async move {
                self.run_task(task, &ctx);
            })
        }).collect();
        for h in handles {
            let _ = h.await;
        }
    }
}
```

- [ ] **Step 4: 运行 `cargo build` 确认 Chunk 2 编译通过**

---

## Chunk 3: 工具适配层 & 中间件（~150行）

**Files:**
- Modify: `src/compose/tool_node/mod.rs`

- [ ] **Step 1: 实现 endpoint 包装函数（适配 Synapse 的 async trait → sync 函数）**

```rust
// 将 Arc<dyn InvokableTool> 包装为同步 endpoint
fn wrap_invokable_tool(
    tool: Arc<dyn crate::components::tool::InvokableTool>,
) -> impl Fn(ToolInput) -> Result<ToolOutput, ToolsNodeError> + Send + Sync {
    Arc::new(move |input: ToolInput| {
        let arguments = input.arguments;
        let result = futures::executor::block_on(tool.invoke(arguments));
        match result {
            Ok(s) => Ok(ToolOutput { result: s }),
            Err(e) => Err(ToolsNodeError::Execution { name: input.name, error: e.to_string() }),
        }
    })
}
```

- [ ] **Step 2: 实现 StreamableTool → StreamToolOutput 适配**

- [ ] **Step 3: 实现 `streamableToInvokable` 和 `invokableToStreamable` 互相转换**

- [ ] **Step 4: 实现 `Enhanced` 版本适配**

- [ ] **Step 5: 实现 middleware chain（倒序执行）**

```rust
fn apply_middlewares<F>(middlewares: &[Arc<...>], base: F) -> F
where
    F: Fn(ToolInput) -> Result<ToolOutput, ToolsNodeError> + Send + Sync + 'static
{
    let mut next = base;
    for m in middlewares.iter().rev() {
        next = m(next);
    }
    next
}
```

- [ ] **Step 6: 运行 `cargo build` 确认 Chunk 3 编译通过**

---

## Chunk 4: 测试（~200行）

**Files:**
- Create: `src/compose/tool_node/tests/tools_node_test.rs`
- Modify: `src/compose/tool_node/mod.rs` 添加 `#[cfg(test)]`

- [ ] **Step 1: 写一个简单的 mock `InvokableTool`**

```rust
struct MockTool {
    name: String,
    result: String,
}

impl MockTool {
    fn new(name: &str, result: &str) -> Self { ... }
}

#[async_trait::async_trait]
impl crate::components::tool::InvokableTool for MockTool {
    async fn invoke(&self, arguments: String) -> crate::components::tool::ToolResult_ {
        Ok(self.result.clone())
    }
}
```

- [ ] **Step 2: 测试 `invoke` — 单个工具调用**

```rust
#[tokio::test]
async fn test_tools_node_single_tool() {
    let tool = Arc::new(MockTool::new("get_weather", "sunny"));
    let node = ToolsNode::new(ToolsNodeConfig {
        tools: vec![tool],
        ..Default::default()
    }).unwrap();

    let input = Message::assistant("What is the weather?");
    let input.tool_calls = Some(vec![ToolCall {
        id: "call_1".into(),
        tool_type: "function".into(),
        function: FunctionCall { name: "get_weather".into(), arguments: "{}".into() },
    }]);

    let ctx = Context::new();
    let output = node.invoke(ctx, input).await.unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].tool_call_id.as_deref(), Some("call_1"));
}
```

- [ ] **Step 3: 测试并发执行多个工具**

```rust
#[tokio::test]
async fn test_tools_node_parallel_execution() {
    // 创建两个工具，并发调用
}
```

- [ ] **Step 4: 测试 `execute_sequentially` 模式**

- [ ] **Step 5: 测试 `UnknownToolsHandler`**

```rust
#[tokio::test]
async fn test_unknown_tool_handler() {
    let handler = Arc::new(|name: String, _input: String| Ok(format!("unknown tool: {}", name)));
    let node = ToolsNode::new(ToolsNodeConfig {
        tools: vec![],
        unknown_tool_handler: Some(handler),
        ..Default::default()
    }).unwrap();
    // 调用不存在的工具，handler 被调用
}
```

- [ ] **Step 6: 测试 `ToolArgumentsHandler`**

- [ ] **Step 7: 运行 `cargo test` 确认全部通过**

---

## Chunk 5: 集成 & 完善

**Files:**
- Modify: `src/compose/tool_node/mod.rs`
- Add: `tool_node` to `compose` module exports

- [ ] **Step 1: 添加 `impl Default for ToolsNodeConfig`**

- [ ] **Step 2: 确保 `src/compose/mod.rs` 正确 re-export ToolsNode**

- [ ] **Step 3: 运行完整 `cargo build && cargo test`**

- [ ] **Step 4: 如果有诊断错误，修复**

- [ ] **Step 5: git commit**
