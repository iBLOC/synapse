---
title: Add Tools to an Agent
---

# 给 Agent 添加工具

让你的 agent 能够执行动作（搜索、计算、获取数据）。

## 目标

构建一个带 `Calculator` tool 的 agent，供 LLM 调用。

## 第 1 步：定义 Tool

```rust
use std::sync::Arc;
use async_trait::async_trait;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use synapse::compose::tool_node::{ToolsNode, ToolsNodeConfig};
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

## 第 2 步：构建 Agent

```rust
let http = create_default_http_client();
    let model = Arc::new(OpenAIModel::new(http, "sk-..."));

let tools_node = Arc::new(ToolsNode::new(ToolsNodeConfig {
    tools: vec![Arc::new(Calculator)],
    ..Default::default()
})?);

let agent = GraphReActAgent::builder()
    .name("math-agent")
    .with_model(model)
    .with_tools_node(tools_node)
    .max_steps(5)
    .build()?;
```

## 第 3 步：测试

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

## 添加多个 Tools

```rust
let agent = GraphReActAgent::builder()
    .with_model(model)
    .with_tool(Arc::new(Calculator))
    .with_tool(Arc::new(WebSearch))
    .with_tool(Arc::new(CalendarReader))
    .build()?;
```

LLM 根据用户的问题自行决定调用哪个 tool。

## Next Steps
