use crate::schema::Message;
use async_trait::async_trait;

pub mod react;

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
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

    async fn resume(&self, checkpoint: Vec<u8>, info: ResumeInfo) -> AgentResult<AgentEvent>;
}

pub struct ResumeInfo {
    pub user_input: String,
}
