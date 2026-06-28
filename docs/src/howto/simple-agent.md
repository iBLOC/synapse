# Build a Simple Agent

A minimal LLM agent in 30 lines of Rust.

## Goal

Create an agent that takes a user message and returns an LLM response.

## Code

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up the model
    let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

    // 2. Build the agent
    let agent = GraphReActAgent::builder()
        .name("assistant")
        .description("A helpful AI assistant")
        .with_model(model)
        .max_steps(5)
        .build()?;

    // 3. Prepare input
    let input = AgentInput {
        messages: vec![Message::user("What is the meaning of life?")],
        enable_streaming: false,
    };

    // 4. Run
    let event = agent.run(input).await?;

    // 5. Get response
    if let Some(msg) = event.output.message {
        println!("Response: {:?}", msg.content);
    }

    Ok(())
}
```

## What's happening?

1. `OpenAIModel::new("gpt-4", key)` — Wraps OpenAI's API
2. `GraphReActAgent::builder()` — Constructs a ReAct agent
3. `.max_steps(5)` — Limit the agent to 5 LLM/tool iterations
4. `agent.run(input)` — Executes the agent loop
5. `event.output.message` — The final assistant message

## Next Steps

- [Add Tools to an Agent](./agent-with-tools.md) — Enable tool use
- [Multi-turn ReAct Agent](./react-agent.md) — Conversational agent
