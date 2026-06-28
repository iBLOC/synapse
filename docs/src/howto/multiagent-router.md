# MultiAgent: Router/Supervisor

Route tasks to specialized sub-agents using an LLM.

## Goal

Build a multi-agent system where a top-level LLM picks the right
specialist for each user query.

## Use Case

- Customer service (billing, refund, tech support)
- Research assistant (search, summarization, citation)
- Code helper (debug, refactor, document)

## Code

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::RouterSupervisor;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

// Define sub-agents
let researcher: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("researcher", "Researches topics", model.clone())
        .with_tools(vec![Arc::new(WebSearch)])
        .build()
);

let writer: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("writer", "Writes content", model.clone())
        .with_tools(vec![Arc::new(MarkdownFormatter)])
        .build()
);

let critic: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("critic", "Reviews and improves content", model.clone())
        .build()
);

// Build supervisor
let supervisor = RouterSupervisor::builder("supervisor", model)
    .description("Coordinates research, writing, and review")
    .add_agent(researcher)
    .add_agent(writer)
    .add_agent(critic)
    .max_iterations(8)
    .build();

// Use
let event = supervisor.run(AgentInput {
    messages: vec![Message::user("Write a blog post about Rust async")],
    enable_streaming: false,
}).await?;
```

## How It Works

1. User asks question
2. LLM sees available agents as `route_to_<name>` tools
3. LLM picks one, calls its tool
4. Sub-agent runs, returns result
5. LLM sees result, decides:
   - Call another sub-agent (e.g., researcher → writer → critic)
   - Synthesize final answer

## System Prompt

The default system prompt is:

> You are a supervisor that coordinates specialized agents to answer user questions. Available agents (each is a tool): {agent_descriptions}. When the user asks a question: 1. Decide which agent is best suited 2. Call that agent's tool with a clear "task" description 3. If result is incomplete, call another agent 4. When done, provide a final synthesized answer (without tool calls).

## Custom System Prompt

```rust
let supervisor = RouterSupervisor::builder("supervisor", model)
    .system_prompt("You are a research assistant coordinator. Always start with the researcher, then have the writer format the findings, then have the critic review.")
    .add_agent(researcher)
    .add_agent(writer)
    .add_agent(critic)
    .build();
```

## max_iterations

- **Default:** 8
- **Low (3-5):** Faster, but LLM has fewer chances to refine
- **High (15+):** More sophisticated workflows, but slower

## Nesting

Sub-agents can themselves be `RouterSupervisor` (nested routing):

```rust
let inner_supervisor = RouterSupervisor::builder("inner", model.clone())
    .add_agent(sub_agent_a)
    .add_agent(sub_agent_b)
    .build();

let outer_supervisor = RouterSupervisor::builder("outer", model)
    .add_agent(Arc::new(inner_supervisor))  // nested
    .add_agent(other_agent)
    .build();
```

## See Also

- [MultiAgent concepts](../concepts/multiagent.md)
- [Orchestrator how-to](./multiagent-orchestrator.md) — Parallel dispatch
- [Handoff how-to](./multiagent-handoff.md) — Conversational transfer
