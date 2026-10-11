---
title: MultiAgent
---

# MultiAgent

Multi-agent systems coordinate multiple specialized agents to solve
complex tasks. Synapse supports 4 distinct coordination patterns.

## Pattern Overview

| Pattern | Coordinator | Signal | Use Case |
|---------|-------------|--------|----------|
| **Router/Supervisor** | Top-level LLM | Tool call | Single-task classification, customer service |
| **Host/Orchestrator** | Top-level LLM | Parallel tool calls | Research, multi-perspective analysis |
| **Conversational Handoff** | None (sub-agents decide) | `event.action.transfer_to` | Multi-role conversations |
| **Hybrid** | LLM + handoff / LLM + handoff | Both | Complex routing with dynamic transfers |

## Router/Supervisor

A top-level LLM agent routes tasks to specialized sub-agents. The
supervisor's LLM calls `route_to_<agent_name>` tools to dispatch.

```rust
let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .max_iterations(8)
    .build();
```

**Flow:**
1. User asks question
2. LLM picks sub-agent via tool call
3. Sub-agent runs, returns result
4. LLM sees result, decides: call another agent or synthesize

## Host/Orchestrator

Decomposes complex tasks and dispatches sub-agents **in parallel**.

```rust
let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .add_agent(researcher_agent)
    .add_agent(analyst_agent)
    .add_agent(writer_agent)
    .max_iterations(12)
    .build();
```

**Flow:**
1. User asks complex question
2. LLM decomposes + emits 3 parallel tool calls
3. System runs all 3 sub-agents in parallel
4. LLM sees all 3 results, synthesizes final answer

## Conversational Handoff

Sub-agents decide when to hand off. No top-level LLM.

```rust
let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .add_agent("tech", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
```

**Flow:**
1. Entry agent runs (billing)
2. Billing returns `event.action.transfer_to = "refund"`
3. Supervisor runs refund
4. Refund returns no transfer → final answer

**How an agent signals transfer**

```rust
async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
    Ok(AgentEvent {
        agent_name: self.name.clone(),
        output: AgentOutput {
            message: Some(Message::assistant("Transferring to refund")),
            action: Some(AgentAction {
                exit: false,
                transfer_to: Some("refund".into()),
                interrupted: None,
            }),
        },
        error: None,
    })
}
```

## Hybrid (Router + Handoff)

LLM picks initial agent, sub-agents can transfer, then LLM synthesizes.

```rust
let hybrid = HybridSupervisor::builder("hybrid", model)
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .max_handoffs(5)
    .build();
```

## Nesting

MultiAgent supervisors can be nested. A `RouterSupervisor` can have a
`HandoffSupervisor` as one of its sub-agents, etc.

## See Also

- [Router how-to](/howto/multiagent-router)
- [Orchestrator how-to](/howto/multiagent-orchestrator)
- [Handoff how-to](/howto/multiagent-handoff)
- [Hybrid how-to](/howto/multiagent-hybrid)
