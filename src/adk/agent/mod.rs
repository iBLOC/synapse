use crate::schema::Message;
use async_trait::async_trait;

pub mod react;

pub use crate::adk::stream::{AgentStream, AgentStreamEvent};

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("agent error: {0}")]
    Agent(String),
    #[error("max iterations exceeded")]
    MaxIterationsExceeded,
    #[error("tool error: {0}")]
    ToolError(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("interrupted")]
    Interrupted,
}

pub struct AgentInput {
    pub messages: Vec<Message>,
    pub enable_streaming: bool,
}

pub struct AgentOutput {
    pub message: Option<Message>,
    pub action: Option<AgentAction>,
}

pub struct AgentAction {
    pub exit: bool,
    pub transfer_to: Option<String>,
    pub interrupted: Option<InterruptInfo>,
}

pub struct InterruptInfo {
    pub message: String,
    pub checkpoint: Vec<u8>,
}

pub struct AgentEvent {
    pub agent_name: String,
    pub output: AgentOutput,
    pub error: Option<AgentError>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent>;

    /// Stream execution events. Default implementation buffers `run()` and
    /// emits a single terminal [`AgentStreamEvent::Final`] (or
    /// [`AgentStreamEvent::Error`]). Concrete agents with incremental output
    /// (e.g. `ChatModelAgent`) override for true token-level streaming.
    async fn stream_run(&self, input: AgentInput) -> AgentStream {
        let name = self.name();
        let agent_name = name.clone();
        let result = self.run(input).await;
        let (writer, reader) = crate::stream::StreamReader::<AgentStreamEvent>::new(8);
        tokio::spawn(async move {
            match result {
                Ok(event) => {
                    let _ = writer
                        .send(AgentStreamEvent::Final {
                            agent: agent_name,
                            message: event
                                .output
                                .message
                                .unwrap_or_else(|| Message::user("")),
                        })
                        .await;
                }
                Err(err) => {
                    let _ = writer
                        .send(AgentStreamEvent::Error {
                            agent: agent_name,
                            error: err,
                        })
                        .await;
                }
            }
        });
        reader
    }

    async fn resume(&self, checkpoint: Vec<u8>, info: ResumeInfo) -> AgentResult<AgentEvent>;
}

pub struct ResumeInfo {
    pub user_input: String,
}
