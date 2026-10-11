---
title: MultiAgent
---

# MultiAgent · 多 Agent 系统

Multi-agent systems coordinate multiple specialized agents to solve
complex tasks. Synapse supports 4 distinct coordination patterns.
*多 agent 系统协调多个专门化 agent 来解决复杂任务。Synapse 支持 4 种不同的协调模式。*

## Pattern Overview · 模式总览

| Pattern / 模式 | Coordinator / 协调者 | Signal / 信号 | Use Case / 适用场景 |
|---------|-------------|--------|----------|
| **Router/Supervisor** / 路由式 | Top-level LLM / 顶层 LLM | Tool call / 工具调用 | Single-task classification, customer service / 单任务分类、客服 |
| **Host/Orchestrator** / 编排式 | Top-level LLM / 顶层 LLM | Parallel tool calls / 并行工具调用 | Research, multi-perspective analysis / 研究、多视角分析 |
| **Conversational Handoff** / 对话式 Handoff | None (sub-agents decide) / 无（由 sub-agent 决定） | `event.action.transfer_to` | Multi-role conversations / 多角色对话 |
| **Hybrid** / 混合式 | LLM + handoff / LLM + handoff | Both / 两者 | Complex routing with dynamic transfers / 复杂路由 + 动态移交 |

## Router/Supervisor · 路由式调度

A top-level LLM agent routes tasks to specialized sub-agents. The
supervisor's LLM calls `route_to_<agent_name>` tools to dispatch.
*顶层 LLM agent 把任务路由给专门化的 sub-agent。supervisor 的 LLM 调用 `route_to_<agent_name>` 工具来完成分发。*

```rust
let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .max_iterations(8)
    .build();
```

**Flow / 流程：**
1. User asks question
   *用户提问*
2. LLM picks sub-agent via tool call
   *LLM 通过 tool call 选择 sub-agent*
3. Sub-agent runs, returns result
   *sub-agent 运行并返回结果*
4. LLM sees result, decides: call another agent or synthesize
   *LLM 看到结果，决定：再调用另一个 agent 还是综合回答*

## Host/Orchestrator · 主控/编排

Decomposes complex tasks and dispatches sub-agents **in parallel**.
*将复杂任务分解，并**并行**分发 sub-agent。*

```rust
let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .add_agent(researcher_agent)
    .add_agent(analyst_agent)
    .add_agent(writer_agent)
    .max_iterations(12)
    .build();
```

**Flow / 流程：**
1. User asks complex question
   *用户提出复杂问题*
2. LLM decomposes + emits 3 parallel tool calls
   *LLM 分解任务并发出 3 个并行 tool call*
3. System runs all 3 sub-agents in parallel
   *系统并行运行全部 3 个 sub-agent*
4. LLM sees all 3 results, synthesizes final answer
   *LLM 看到 3 个结果后综合出最终回答*

## Conversational Handoff · 对话式 Handoff

Sub-agents decide when to hand off. No top-level LLM.
*由 sub-agent 自行决定何时 hand off。没有顶层 LLM。*

```rust
let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .add_agent("tech", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
```

**Flow / 流程：**
1. Entry agent runs (billing)
   *入口 agent 运行（billing）*
2. Billing returns `event.action.transfer_to = "refund"`
   *billing 返回 `event.action.transfer_to = "refund"`*
3. Supervisor runs refund
   *supervisor 运行 refund*
4. Refund returns no transfer → final answer
   *refund 不再移交 → 给出最终回答*

**How an agent signals transfer / agent 如何发出移交信号：**

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

## Hybrid (Router + Handoff) · 混合（Router + Handoff）

LLM picks initial agent, sub-agents can transfer, then LLM synthesizes.
*LLM 选定初始 agent，sub-agent 之间可以移交，最后由 LLM 综合回答。*

```rust
let hybrid = HybridSupervisor::builder("hybrid", model)
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .max_handoffs(5)
    .build();
```

## Nesting · 嵌套

MultiAgent supervisors can be nested. A `RouterSupervisor` can have a
`HandoffSupervisor` as one of its sub-agents, etc.
*MultiAgent supervisor 可以嵌套。例如 `RouterSupervisor` 的 sub-agent 之一可以是 `HandoffSupervisor`，等等。*

## See Also · 相关阅读

- [Router how-to](/howto/multiagent-router)
- [Orchestrator how-to](/howto/multiagent-orchestrator)
- [Handoff how-to](/howto/multiagent-handoff)
- [Hybrid how-to](/howto/multiagent-hybrid)
