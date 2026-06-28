# Agent Trait

The core abstraction. Every autonomous unit implements this trait.

## Definition

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent>;

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: ResumeInfo,
    ) -> AgentResult<AgentEvent>;
}
```

## Types

```rust
pub struct AgentInput {
    pub messages: Vec<Message>,
    pub enable_streaming: bool,
}

pub struct AgentEvent {
    pub agent_name: String,
    pub output: AgentOutput,
    pub error: Option<AgentError>,
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

pub struct ResumeInfo {
    pub user_input: String,
}

pub enum AgentError {
    Agent(String),
    MaxIterationsExceeded,
    ToolError(String),
    InvalidInput(String),
    Interrupted,
}
```

## Usage

```rust
use std::sync::Arc;
use synapse::adk::agent::Agent;

let agent: Arc<dyn Agent> = Arc::new(my_agent);
let event = agent.run(input).await?;
```

## See Also

- [Agent concepts](../concepts/agent.md)
- [MultiAgent patterns](./multiagent-patterns.md)
