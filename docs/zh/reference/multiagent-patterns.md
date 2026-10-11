---
title: MultiAgent Patterns
---

# MultiAgent 模式

Synapse 支持 4 种 MultiAgent 协调模式。它们都实现标准的 `Agent` trait，因此可以互相替换。

## 模式对比

| 模式 | 协调者 | 信号 | 适用场景 |
|---------|-------------|--------|----------|
| `RouterSupervisor` | LLM | 工具调用 | 单任务分类 |
| `OrchestratorAgent` | LLM | 并行工具调用 | 研究、多视角 |
| `HandoffSupervisor` | 基于规则 | `event.action.transfer_to` | 多角色对话 |
| `HybridSupervisor` | LLM + handoff / LLM + handoff | 两者 | 带升级的复杂流程 |

## API 快速参考

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

## 嵌套

所有模式都可以嵌套。例如 `RouterSupervisor` 的 sub-agent 之一可以是 `HandoffSupervisor`，等等。

## 相关阅读
