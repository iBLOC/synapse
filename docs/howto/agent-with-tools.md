---
title: Add Tools to an Agent
---

# Add Tools to an Agent · 给 Agent 添加工具

Enable your agent to perform actions (search, calculate, fetch data).
*让你的 agent 能够执行动作（搜索、计算、获取数据）。*

## Goal · 目标

Build an agent with a `Calculator` tool that the LLM can call.
*构建一个带 `Calculator` tool 的 agent，供 LLM 调用。*

## Step 1: Define the Tool · 第 1 步：定义 Tool

```rust
use std::sync::Arc;
use async_trait::async_trait;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use synapse::schema::ToolInfo;

struct Calculator;

#[async_trait]
impl BaseTool for Calculator {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move {
            Ok(ToolInfo {
                name: "calculator".into(),
                description: "Evaluates a basic arithmetic expression.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": {"type": "string"}
                    },
                    "required": ["expression"]
                }),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for Calculator {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        // Simple evaluation
        Ok("42".to_string())
    }
}
```

## Step 2: Build the Agent · 第 2 步：构建 Agent

```rust
let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));
let calc = Arc::new(Calculator);

let agent = GraphReActAgent::builder()
    .name("math-agent")
    .with_model(model)
    .with_tool(calc)
    .max_steps(5)
    .build()?;
```

## Step 3: Test · 第 3 步：测试

```rust
let input = AgentInput {
    messages: vec![Message::user("What is 5 + 3?")],
    enable_streaming: false,
};
let event = agent.run(input).await?;
// Agent will:
// 1. See the question
// 2. Decide to call calculator tool
// 3. Get result "8"
// 4. Return "5 + 3 = 8"
```

## Adding Multiple Tools · 添加多个 Tools

```rust
let agent = GraphReActAgent::builder()
    .with_model(model)
    .with_tool(Arc::new(Calculator))
    .with_tool(Arc::new(WebSearch))
    .with_tool(Arc::new(CalendarReader))
    .build()?;
```

The LLM picks which tool to call based on the user's question.
*LLM 根据用户的问题自行决定调用哪个 tool。*

## Next Steps

- [Multi-turn ReAct Agent](/howto/react-agent) — Handle conversations
- [Connect to MCP Server](/howto/mcp-server) — Use external tools
