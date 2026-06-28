# MultiAgent: Orchestrator

Decompose complex tasks and dispatch sub-agents in parallel.

## Goal

Build a multi-agent system where the LLM decomposes a task and
dispatches multiple sub-agents in a single response (parallel
execution).

## When to Use

- Research tasks (gather from multiple sources in parallel)
- Multi-perspective analysis (analyze from different angles)
- Workflows with independent sub-tasks

## Code

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::OrchestratorAgent;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

// Specialized agents
let researcher = Arc::new(researcher_agent);
let analyst = Arc::new(analyst_agent);
let writer = Arc::new(writer_agent);

let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .description("Decomposes tasks and dispatches specialists in parallel")
    .add_agent(researcher)
    .add_agent(analyst)
    .add_agent(writer)
    .max_iterations(12)
    .build();

let event = orchestrator.run(AgentInput {
    messages: vec![Message::user(
        "Research the impact of AI on healthcare, \
         analyze the economic implications, and write a summary report."
    )],
    enable_streaming: false,
}).await?;
```

## How It Works

1. LLM sees complex task
2. LLM decomposes into 3 independent sub-tasks
3. LLM emits 3 tool calls in **one response** (parallel)
4. System runs all 3 sub-agents in parallel
5. LLM sees all 3 results, synthesizes final answer

## Difference from Router

| Pattern | LLM Behavior |
|---------|--------------|
| Router | Sequential, "call A, then call B" |
| Orchestrator | Parallel, "call A, B, C all at once" |

Modern LLMs (GPT-4, Claude 3) support parallel tool calls natively.

## System Prompt

The orchestrator prompt explicitly says:

> Call the appropriate agent tools in PARALLEL — make multiple tool calls in a SINGLE response to dispatch all sub-tasks at once.

## Limitations

- Sub-agents must be **independent** (no dependencies between them)
- If dependencies exist, use sequential Router instead
- LLM must support parallel tool calls (all modern LLMs do)

## See Also

- [Router how-to](./multiagent-router.md) — Sequential routing
- [MultiAgent concepts](../concepts/multiagent.md)
