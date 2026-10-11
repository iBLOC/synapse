---
title: Multi-turn ReAct Agent
---

# Multi-turn ReAct Agent · 多轮 ReAct Agent

Handle multi-turn conversations with memory and tool use.
*带记忆与 tool 调用的多轮对话处理。*

## Goal · 目标

Build a conversational agent that maintains message history across
turns and uses tools as needed.
*构建一个跨轮次维护消息历史、按需调用 tool 的对话式 agent。*

## Approach · 思路

Synapse's `ChatModelAgent` and `GraphReActAgent` both maintain message
history automatically. You just need to pass the full message list
each time.
*Synapse 的 `ChatModelAgent` 和 `GraphReActAgent` 都会自动维护消息历史，你只需每次传入完整的消息列表。*

## Code · 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;
use synapse::schema::Message;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));
let agent = GraphReActAgent::builder()
    .name("assistant")
    .with_model(model)
    .with_tool(Arc::new(Calculator))
    .max_steps(10)
    .build()?;

// Maintain history
let mut history: Vec<Message> = Vec::new();

// Turn 1
history.push(Message::user("What is 5 + 3?"));
let event = agent.run(AgentInput {
    messages: history.clone(),
    enable_streaming: false,
}).await?;
if let Some(msg) = event.output.message {
    history.push(msg);
}

// Turn 2 (LLM has context of Turn 1)
history.push(Message::user("Multiply that by 2"));
let event = agent.run(AgentInput {
    messages: history.clone(),
    enable_streaming: false,
}).await?;
```

## With Session Management · 结合 Session 管理

For persistent sessions, use `Runner`:
*要持久化 session，用 `Runner`：*

```rust
use synapse::adk::runner::Runner;

let runner = Runner::new(Box::new(agent));

// In a web server, store history per session ID
let session_id = "user-123";
let history = session_store.get_or_create(session_id).await;

let event = runner.run(AgentInput {
    messages: history,
    enable_streaming: false,
}).await?;

if let Some(msg) = event.output.message {
    session_store.append(session_id, msg).await;
}
```

## max_steps Tradeoff · max_steps 权衡

- **Low (3-5):** Faster, but may fail on complex multi-tool tasks
  *低（3-5）：更快，但复杂的多 tool 任务可能失败*
- **High (10-20):** More capable, but slower and may loop
  *高（10-20）：能力更强，但更慢且可能陷入循环*

For most applications, `max_steps = 5-10` is a good default.
*对多数应用来说，`max_steps = 5-10` 是不错的默认值。*

## Next Steps · 下一步

- [Build a RAG Pipeline](/howto/rag-pipeline) — Add knowledge retrieval
  *加入知识检索*
- [MultiAgent Router](/howto/multiagent-router) — Multiple specialized agents
  *多个各司其职的 agent*
