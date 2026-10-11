---
title: MultiAgent Patterns
---

# MultiAgent Patterns · MultiAgent 模式

Synapse supports 4 MultiAgent coordination patterns. All implement
the standard `Agent` trait, so they're interchangeable.
*Synapse 支持 4 种 MultiAgent 协调模式。它们都实现标准的 `Agent` trait，因此可以互相替换。*

## Pattern Comparison · 模式对比

| Pattern / 模式 | Coordinator / 协调者 | Signal / 信号 | Best For / 适用场景 |
|---------|-------------|--------|----------|
| `RouterSupervisor` | LLM | Tool call / 工具调用 | Single-task classification / 单任务分类 |
| `OrchestratorAgent` | LLM | Parallel tool calls / 并行工具调用 | Research, multi-perspective / 研究、多视角 |
| `HandoffSupervisor` | Rule-based / 基于规则 | `event.action.transfer_to` | Multi-role conversations / 多角色对话 |
| `HybridSupervisor` | LLM + handoff / LLM + handoff | Both / 两者 | Complex flows with escalation / 带升级的复杂流程 |

## Quick API Reference · API 快速参考

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

## Nesting · 嵌套

All patterns can be nested. A `RouterSupervisor` can have a
`HandoffSupervisor` as a sub-agent, etc.
*所有模式都可以嵌套。例如 `RouterSupervisor` 的 sub-agent 之一可以是 `HandoffSupervisor`，等等。*

## See Also · 相关阅读

- [MultiAgent concepts](/concepts/multiagent)
- [How-to guides](/howto/multiagent-router)
