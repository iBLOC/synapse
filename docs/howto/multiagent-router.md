---
title: MultiAgent Router
---

# MultiAgent: Router/Supervisor · MultiAgent：Router/Supervisor

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

let researcher: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("researcher", "Researches topics", model.clone())
        .with_tools(vec![Arc::new(WebSearch)])
        .build()
);

let writer: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("writer", "Writes content", model.clone())
        .build()
);

let critic: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("critic", "Reviews content", model.clone())
        .build()
);

let supervisor = RouterSupervisor::builder("supervisor", model)
    .description("Coordinates research, writing, and review")
    .add_agent(researcher)
    .add_agent(writer)
    .add_agent(critic)
    .max_iterations(8)
    .build();

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
5. LLM sees result, decides: call another agent or synthesize

## Nesting

Sub-agents can themselves be `RouterSupervisor` (nested routing).

## See Also

- [MultiAgent concepts](/concepts/multiagent)
- [Orchestrator how-to](/howto/multiagent-orchestrator) — Parallel dispatch
- [Handoff how-to](/howto/multiagent-handoff) — Conversational transfer
