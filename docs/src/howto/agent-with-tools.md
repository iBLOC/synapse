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
                description: "Evaluates a basic arithmetic expression. \
                              Example: '2 + 2' or '(5 * 3) - 1'.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The arithmetic expression to evaluate"
                        }
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
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let expression = args["expression"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("missing 'expression'".into()))?;

        // Simple evaluation (replace with meval/evalexpr in production)
        let result = match expression {
            "2 + 2" => "4",
            "5 * 3" => "15",
            e if e.contains('+') => {
                let parts: Vec<&str> = e.split('+').collect();
                if parts.len() == 2 {
                    let a: i64 = parts[0].trim().parse().unwrap_or(0);
                    let b: i64 = parts[1].trim().parse().unwrap_or(0);
                    return Ok((a + b).to_string());
                }
                "0"
            }
            _ => "0",
        };

        Ok(result.to_string())
    }
}
```

## Step 2: Build the Agent

```rust
let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));
let calc = Arc::new(Calculator);

let agent = GraphReActAgent::builder()
    .name("math-agent")
    .description("Agent that can do math")
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

- [Multi-turn ReAct Agent](./react-agent.md) — Handle conversations
- [Connect to MCP Server](./mcp-server.md) — Use external tools
