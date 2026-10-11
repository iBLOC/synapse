---
title: MultiAgent Patterns
---

# MultiAgent Patterns

Synapse supports 4 MultiAgent coordination patterns. All implement
the standard `Agent` trait, so they're interchangeable.

## Pattern Comparison

| Pattern | Coordinator | Signal | Best For |
|---------|-------------|--------|----------|
| `RouterSupervisor` | LLM | Tool call | Single-task classification |
| `OrchestratorAgent` | LLM | Parallel tool calls | Research, multi-perspective |
| `HandoffSupervisor` | Rule-based | `event.action.transfer_to` | Multi-role conversations |
| `HybridSupervisor` | LLM + handoff / LLM + handoff | Both | Complex flows with escalation |

## Quick API Reference

```rust
// Router
RouterSupervisor::builder("name", model)
    .add_agent(agent_a)
    .add_agent(agent_b)
    .build();

// Orchestrator
OrchestratorAgent::builder("name", model)
    .add_agent(agent_a)
    .add_agent(agent_b)
    .add_agent(agent_c)
    .build();

// Handoff
HandoffSupervisor::builder("name")
    .add_agent("a", agent_a)
    .add_agent("b", agent_b)
    .entry_point("a")
    .max_handoffs(5)
    .build();

// Hybrid
HybridSupervisor::builder("name", model)
    .add_agent("a", agent_a)
    .add_agent("b", agent_b)
    .max_handoffs(5)
    .build();
```

## Nesting

All patterns can be nested. A `RouterSupervisor` can have a
`HandoffSupervisor` as a sub-agent, etc.

## See Also

- [MultiAgent concepts](/concepts/multiagent)
- [How-to guides](/howto/multiagent-router)
