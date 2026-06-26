# Phase 2: ToolsNode + Callback 系统

**Goal**: 补全 tool invocation 核心 + callback 系统，作为 ReAct agent 的基础

## 现状分析

### Synapse 已有
- `components/tool/mod.rs`: BaseTool, InvokableTool, StreamableTool, EnhancedInvokableTool, EnhancedStreamableTool traits (骨架)
- `callbacks/mod.rs`: CallbackHandler stub (on_start/on_end/on_error)
- `providers/openai.rs`: LLM provider with chat() 实现
- `adk/react.rs`: ChatModelAgent<ReAct> 半实现

### Eino 有
- 1051行 ToolsNode：完整工具调用处理、middleware chain、parallel execution、enhanced/stream variants
- 完整 callback 系统：HandlerBuilder, AspectInject, TimingChecker
- RunContext：context 传递和 callback 注入

---

## Chunk 1: Callback 完整系统

### Task 1: Callback Handler 完整实现

**文件**:
- 修改: `src/callbacks/mod.rs`

**目标**: 实现完整的 callback hook 系统

```rust
// hooks 生命周期
on_start(ctx, info)           // 节点开始
on_end(ctx, info, output)    // 节点结束
on_error(ctx, info, err)     // 节点出错
on_retry(ctx, info, err)     // 重试前
on_tool_start(ctx, info)      // 工具开始
on_tool_end(ctx, info, result) // 工具结束
on_chat_model_start(ctx, info) // LLM 开始
on_chat_model_end(ctx, info, response) // LLM 结束
```

### Task 2: CallbackManager 实现

```rust
pub struct CallbackManager {
    handlers: Vec<Arc<dyn CallbackHandler>>,
    global_handlers: Vec<Arc<dyn CallbackHandler>>,
}

impl CallbackManager {
    pub fn new() -> Self
    pub fn add_handler(&mut self, handler: Arc<dyn CallbackHandler>)
    pub fn add_global_handler(&mut self, handler: Arc<dyn CallbackHandler>)
    pub async fn run_start(&self, ctx: &Context, info: &RunInfo)
    pub async fn run_end(&self, ctx: &Context, info: &RunInfo, output: &Value)
    pub async fn run_error(&self, ctx: &Context, info: &RunInfo, err: &str)
}
```

---

## Chunk 2: ToolsNode 完整实现

### Task 3: ToolsNode 结构

**文件**:
- 创建: `src/compose/node/tools_node.rs`

```rust
pub struct ToolNode {
    tuple: ToolTuple,
    execute_sequentially: bool,
    tool_arguments_handler: Option<Box<dyn Fn(Context, String, String) -> Result<String, ToolError> + Send + Sync>>,
    middleware: Vec<Box<dyn ToolMiddleware + Send + Sync>>,
}

pub struct ToolInput {
    pub name: String,
    pub arguments: String,  // JSON string
    pub call_id: String,
}

pub struct ToolOutput {
    pub result: String,
    pub is_error: bool,
}
```

### Task 4: Tool 执行逻辑

- 标准调用: `InvokableTool::invoke()` → JSON string → result
- Enhanced 调用: arguments 包装为 `ToolArgument`, 返回 `ToolResult`
- Parallel execution: 使用 `tokio::spawn` 并发执行多个工具
- Middleware chain: reverse order 应用

### Task 5: 集成到 Graph

- 实现 `ToolsNode as Runnable<serde_json::Value, serde_json::Value>`
- 添加到 `compose/graph/mod.rs` 的节点类型支持
- 支持 `add_tools_node()` 方法

---

## Chunk 3: ReAct Agent 完善

### Task 6: 完善 ChatModelAgent

- 实现完整的 ReAct loop (Thought/Action/Observation)
- 集成 ToolsNode 进行工具调用
- 实现 `max_iterations` 限制
- 实现 tool result 解析和格式化为 message

---

## 执行顺序

1. **Chunk 1** (Task 1-2): Callback 系统
2. **Chunk 2** (Task 3-5): ToolsNode 实现
3. **Chunk 3** (Task 6): ReAct Agent 完善
