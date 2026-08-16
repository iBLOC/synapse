---
title: Add Tools to an Agent
---

# Add Tools to an Agent

Enable your agent to perform actions (search, calculate, fetch data).

## Goal

Build an agent with a `Calculator` tool that the LLM can call.

## Step 1: Define the Tool

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

## Step 2: Build the Agent

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

## Step 3: Test

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

## Adding Multiple Tools

```rust
let agent = GraphReActAgent::builder()
    .with_model(model)
    .with_tool(Arc::new(Calculator))
    .with_tool(Arc::new(WebSearch))
    .with_tool(Arc::new(CalendarReader))
    .build()?;
```

The LLM picks which tool to call based on the user's question.

## Next Steps

- [Multi-turn ReAct Agent](/howto/react-agent) — Handle conversations
- [Connect to MCP Server](/howto/mcp-server) — Use external tools
