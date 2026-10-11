//! Multi-turn ReAct conversation: GraphReActAgent keeps per-call state, so
//! the caller owns the history — clone it, push the assistant reply back,
//! and every turn sees the full context.
//!
//! Run:
//!   OPENAI_API_KEY=sk-... cargo run --example react_agent

use std::sync::Arc;

use synapse::adk::agent::react::GraphReActAgent;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::components::model::OpenAIModel;
use synapse::llm::create_default_http_client;
use synapse::schema::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = create_default_http_client();
    let model = Arc::new(OpenAIModel::new(http, "sk-..."));

    let agent = GraphReActAgent::builder()
        .name("assistant")
        .description("A conversational assistant with memory")
        .with_model(model)
        .max_steps(10)
        .build()?;

    // The caller owns the history across turns.
    let mut history: Vec<Message> = Vec::new();

    // Turn 1
    history.push(Message::user("What is 5 + 3?"));
    let event = agent
        .run(AgentInput {
            messages: history.clone(),
            enable_streaming: false,
        })
        .await?;
    if let Some(msg) = event.output.message {
        println!("Assistant: {:?}", msg.content);
        history.push(msg);
    }

    // Turn 2 — the model sees Turn 1 in `history`.
    history.push(Message::user("Multiply that by 2"));
    let event = agent
        .run(AgentInput {
            messages: history.clone(),
            enable_streaming: false,
        })
        .await?;
    if let Some(msg) = event.output.message {
        println!("Assistant: {:?}", msg.content);
    }

    Ok(())
}
