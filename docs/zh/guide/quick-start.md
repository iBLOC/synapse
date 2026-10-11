---
title: Quick Start
---

# 快速开始

5 分钟让 Synapse 跑起来。

## 安装

把 Synapse 加进 `Cargo.toml`：

```toml
[dependencies]
synapse = "0.1"
tokio = { version = "1", features = ["full"] }
```

## 你的第一个 Agent

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Arc::new(OpenAIModel::new("gpt-4", "your-openai-key"));

    let agent = GraphReActAgent::builder()
        .name("assistant")
        .description("A helpful AI assistant")
        .with_model(model)
        .max_steps(5)
        .build()?;

    let input = AgentInput {
        messages: vec![Message::user("Hello, who are you?")],
        enable_streaming: false,
    };

    let event = agent.run(input).await?;

    if let Some(msg) = event.output.message {
        println!("Agent: {:?}", msg);
    }

    Ok(())
}
```

## 添加 Tool

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
                description: "Performs basic arithmetic".into(),
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
        // Parse and evaluate expression
        Ok("42".to_string())
    }
}

let calc = Arc::new(Calculator);
let agent = GraphReActAgent::builder()
    .name("math-agent")
    .with_model(model)
    .with_tool(calc)
    .build()?;
```

## 运行测试

```bash
cargo test
```

全部 205 个测试无需任何外部依赖即可运行。

## 下一步

- [构建一个简单 Agent](/zh/howto/simple-agent) — 详细分步讲解
- [架构](/zh/concepts/architecture) — 理解框架架构
- [概念：Agent](/zh/concepts/agent) — 深入理解每个抽象
