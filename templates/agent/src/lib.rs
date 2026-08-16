//! Minimal Synapse agent template.
//!
//! This file is a copy-and-edit starting point. Replace the
//! `agent_card` and `run_agent` bodies with your own agent's logic.
//!
//! See `../../../docs/wasm-agent-abi.md` for the WASM ABI contract
//! (host imports, exported symbols, event payload format) this
//! template is designed for. The `synapse::stream` and
//! `synapse::components::model` modules are the natural building
//! blocks for the agent's actual reasoning loop.

use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput};
use synapse::adk::stream::AgentStreamEvent;
use synapse::stream::StreamReader;
use synapse::schema::Message;

/// A trivial agent that always replies with a fixed greeting.
/// Replace this with your real agent.
pub struct GreetingAgent {
    name: String,
}

impl GreetingAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl Agent for GreetingAgent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        "A trivial greeting agent — replace me with your real agent logic.".into()
    }

    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        let reply = format!("Hello! You said: {:?}", input.messages);
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(Message::assistant(reply)),
                action: None,
            },
            error: None,
        })
    }

    async fn stream_run(
        &self,
        _input: AgentInput,
    ) -> StreamReader<AgentStreamEvent> {
        let (writer, reader) = StreamReader::<AgentStreamEvent>::new(2);
        let name = self.name();
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::Final {
                    agent: name,
                    message: Message::assistant("hello from greeting agent"),
                })
                .await;
        });
        reader
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: synapse::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn greeting_agent_responds() {
        let agent = GreetingAgent::new("test");
        let input = AgentInput {
            messages: vec![Message::user("ping")],
            enable_streaming: false,
        };
        let event = agent.run(input).await.expect("run ok");
        let msg = event.output.message.expect("message present");
        match msg.content {
            synapse::schema::Content::Text(s) => assert!(s.contains("ping")),
            _ => panic!("expected Text content"),
        }
    }
}
