//! Interop harness: host a Synapse agent as an A2A server for an external
//! peer (Google ADK) to connect to.
//!
//! Usage: cargo run --example a2a_interop_server -- <port>

use std::sync::Arc;

use synapse::a2a::card::AgentCard;
use synapse::a2a::server::A2AServerBuilder;
use synapse::adk::agent::{Agent, AgentEvent, AgentInput, AgentOutput, AgentStreamEvent};

/// A Synapse agent that answers with a fixed, unmistakable string.
struct SynapseEchoAgent;

#[async_trait::async_trait]
impl Agent for SynapseEchoAgent {
    fn name(&self) -> String {
        "synapse_interop_agent".to_string()
    }

    fn description(&self) -> String {
        "Synapse agent used for A2A interop testing.".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant(
                    "SYNAPSE_REPLIED_OK",
                )),
                action: None,
            },
            error: None,
        })
    }

    async fn stream_run(
        &self,
        _input: AgentInput,
    ) -> synapse::adk::agent::AgentStream {
        let (writer, reader) = synapse::stream::StreamReader::<AgentStreamEvent>::new(4);
        let name = self.name();
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::Final {
                    agent: name,
                    message: synapse::schema::Message::assistant(
                        "SYNAPSE_REPLIED_OK",
                    ),
                })
                .await;
        });
        reader
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: synapse::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        self.run(AgentInput {
            messages: vec![synapse::schema::Message::user("")],
            enable_streaming: false,
        })
        .await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(9099);

    let mut card = AgentCard::new(
        "synapse_interop_agent",
        "Synapse agent used for A2A interop testing.",
        format!("http://127.0.0.1:{port}"),
        "1.0.0",
    );
    // Declare one extension so the peer can exercise the negotiation.
    card.capabilities.extensions = vec!["https://example.com/ext/v1".to_string()];

    let addr = format!("127.0.0.1:{port}")
        .parse()
        .expect("address must parse");
    let builder = A2AServerBuilder::new(Arc::new(SynapseEchoAgent), card);
    let bound = builder.serve_on(addr).await?;
    println!("SYNAPSE_A2A_LISTENING {bound}");
    println!("SYNAPSE_A2A_CARD {}/.well-known/agent-card.json", bound);
    // Keep serving until the process is killed.
    let _ = tokio::signal::ctrl_c().await;
    Ok(())
}
