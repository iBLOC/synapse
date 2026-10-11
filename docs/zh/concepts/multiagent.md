---
title: MultiAgent
---

# 多 Agent 系统

多 agent 系统协调多个专门化 agent 来解决复杂任务。Synapse 支持 4 种不同的协调模式。

## 模式总览

| 模式 | 协调者 | 信号 | 适用场景 |
|---------|-------------|--------|----------|
| 路由式 | 顶层 LLM | 工具调用 | 单任务分类、客服 |
| 编排式 | 顶层 LLM | 并行工具调用 | 研究、多视角分析 |
| 对话式 Handoff | 无（由 sub-agent 决定） | `event.action.transfer_to` | 多角色对话 |
| 混合式 | LLM + handoff / LLM + handoff | 两者 | 复杂路由 + 动态移交 |

## 路由式调度

顶层 LLM agent 把任务路由给专门化的 sub-agent。supervisor 的 LLM 调用 `route_to_<agent_name>` 工具来完成分发。

```rust
let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .max_iterations(8)
    .build();
```

   1. 用户提问
   2. LLM 通过 tool call 选择 sub-agent
   3. sub-agent 运行并返回结果
   4. LLM 看到结果，决定：再调用另一个 agent 还是综合回答

## 主控/编排

将复杂任务分解，并**并行**分发 sub-agent。

```rust
let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .add_agent(researcher_agent)
    .add_agent(analyst_agent)
    .add_agent(writer_agent)
    .max_iterations(12)
    .build();
```

   1. 用户提出复杂问题
   2. LLM 分解任务并发出 3 个并行 tool call
   3. 系统并行运行全部 3 个 sub-agent
   4. LLM 看到 3 个结果后综合出最终回答

## 对话式 Handoff

由 sub-agent 自行决定何时 hand off。没有顶层 LLM。

```rust
let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .add_agent("tech", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
```

   1. 入口 agent 运行（billing）
   2. billing 返回 `event.action.transfer_to = "refund"`
   3. supervisor 运行 refund
   4. refund 不再移交 → 给出最终回答

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

## 混合（Router + Handoff）

LLM 选定初始 agent，sub-agent 之间可以移交，最后由 LLM 综合回答。

```rust
let hybrid = HybridSupervisor::builder("hybrid", model)
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .max_handoffs(5)
    .build();
```

## 嵌套

MultiAgent supervisor 可以嵌套。例如 `RouterSupervisor` 的 sub-agent 之一可以是 `HandoffSupervisor`，等等。

## 相关阅读
