# Multi-turn ReAct Agent

Handle multi-turn conversations with memory and tool use.

## Goal

Build a conversational agent that maintains message history across
turns and uses tools as needed.

## Approach

Synapse's `ChatModelAgent` and `GraphReActAgent` both maintain message
history automatically. You just need to pass the full message list
each time.

## Code

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
    history.push(msg);  // Add assistant response to history
}

// Turn 2 (LLM has context of Turn 1)
history.push(Message::user("Multiply that by 2"));
let event = agent.run(AgentInput {
    messages: history.clone(),
    enable_streaming: false,
}).await?;
if let Some(msg) = event.output.message {
    history.push(msg);
}
```

## With Session Management

For persistent sessions, use `Runner`:

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

## max_steps Tradeoff

- **Low (3-5):** Faster, but may fail on complex multi-tool tasks
- **High (10-20):** More capable, but slower and may loop

For most applications, `max_steps = 5-10` is a good default.

## Next Steps

- [Build a RAG Pipeline](./rag-pipeline.md) — Add knowledge retrieval
- [MultiAgent Router](./multiagent-router.md) — Multiple specialized agents
