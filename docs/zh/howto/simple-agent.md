---
title: Build a Simple Agent
---

# 构建一个简单 Agent

用 30 行 Rust 写一个最小的 LLM agent。

## 目标

创建一个接收用户消息并返回 LLM 响应的 agent。

## 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;
use synapse::llm::create_default_http_client;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = create_default_http_client();
    let model = Arc::new(OpenAIModel::new(http, "sk-..."));

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

## 发生了什么？

   1. 封装 OpenAI 的 API
   2. 构造一个 ReAct agent
   3. 把 agent 限制在最多 5 轮 LLM/tool 迭代
   4. 执行 agent 循环
   5. 最终的 assistant 消息

## 下一步

  - 启用 tool 调用
  - 对话式 agent
