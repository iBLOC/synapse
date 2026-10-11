---
title: Build a Simple Agent
---

# Build a Simple Agent · 构建一个简单 Agent

A minimal LLM agent in 30 lines of Rust.
*用 30 行 Rust 写一个最小的 LLM agent。*

## Goal · 目标

Create an agent that takes a user message and returns an LLM response.
*创建一个接收用户消息并返回 LLM 响应的 agent。*

## Code · 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

    let agent = GraphReActAgent::builder()
        .name("assistant")
        .description("A helpful AI assistant")
        .with_model(model)
        .max_steps(5)
        .build()?;

    let input = AgentInput {
        messages: vec![Message::user("What is the meaning of life?")],
        enable_streaming: false,
    };

    let event = agent.run(input).await?;

    if let Some(msg) = event.output.message {
        println!("Response: {:?}", msg.content);
    }

    Ok(())
}
```

## What's happening? · 发生了什么？

1. `OpenAIModel::new("gpt-4", key)` — Wraps OpenAI's API
   *封装 OpenAI 的 API*
2. `GraphReActAgent::builder()` — Constructs a ReAct agent
   *构造一个 ReAct agent*
3. `.max_steps(5)` — Limit the agent to 5 LLM/tool iterations
   *把 agent 限制在最多 5 轮 LLM/tool 迭代*
4. `agent.run(input)` — Executes the agent loop
   *执行 agent 循环*
5. `event.output.message` — The final assistant message
   *最终的 assistant 消息*

## Next Steps · 下一步

- [Add Tools to an Agent](/howto/agent-with-tools) — Enable tool use
  *启用 tool 调用*
- [Multi-turn ReAct Agent](/howto/react-agent) — Conversational agent
  *对话式 agent*
