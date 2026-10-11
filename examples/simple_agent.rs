//! Minimal LLM agent: an OpenAI model wrapped in a GraphReActAgent, one
//! user message in, one assistant message out.
//!
//! Run (needs an OpenAI API key):
//!   OPENAI_API_KEY=sk-... cargo run --example simple_agent

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
