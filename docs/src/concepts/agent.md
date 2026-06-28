# Agent

The `Agent` trait is the central abstraction in Synapse. Every autonomous
unit — ReAct agent, Router, Orchestrator, Handoff, custom logic — implements
this trait.

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

## Input/Output

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
    pub transfer_to: Option<String>,  // for HandoffSupervisor
    pub interrupted: Option<InterruptInfo>,
}
```

## Built-in Agent Implementations

| Agent | Use Case | Run Loop |
|-------|----------|----------|
| `ChatModelAgent<M, T>` | Single LLM + tools | ReAct loop |
| `GraphReActAgent` | Graph-based orchestration | Graph traversal |
| `RouterSupervisor` | Multi-agent routing | ReAct loop with sub-agent tools |
| `OrchestratorAgent` | Task decomposition | ReAct loop with parallel tool calls |
| `HandoffSupervisor` | Multi-role conversations | Sequential sub-agent calls |
| `HybridSupervisor` | Router + handoff | ReAct + forced handoff |

## Implementing a Custom Agent

```rust
use async_trait::async_trait;
use synapse::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo,
};

struct MyAgent {
    name: String,
    description: String,
}

#[async_trait]
impl Agent for MyAgent {
    fn name(&self) -> String { self.name.clone() }
    fn description(&self) -> String { self.description.clone() }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        // Your logic here
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant("Hello!")),
                action: None,
            },
            error: None,
        })
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not supported".into()))
    }
}
```

## Agent Composition

Since `Agent` is a trait, agents can be composed:

```rust
let outer: Arc<dyn Agent> = Arc::new(my_outer_agent);
let inner: Arc<dyn Agent> = Arc::new(my_inner_agent);

// Pass inner as sub-agent to outer
outer.run(input).await?;
```

MultiAgent patterns leverage this for hierarchical composition.
