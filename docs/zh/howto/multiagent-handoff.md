---
title: MultiAgent Handoff
---

# MultiAgent：会话式 Handoff

由 sub-agent 自行决定何时把会话移交给另一个 agent。

## 目标

构建一个多角色系统，agent 能够根据上下文动态移交会话。

## 使用场景

  - 客服（账单 → 退款 → 技术支持）
  - 多专家会诊
  - 服务台分级派单（L1 → L2 → L3）

## Sub-Agent 如何发出 Handoff 信号

sub-agent 通过设置 `event.action.transfer_to` 来发出 handoff 信号：

```rust
async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
    Ok(AgentEvent {
        agent_name: self.name.clone(),
        output: AgentOutput {
            message: Some(Message::assistant(
                "This appears to be a refund question, transferring"
            )),
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

## Supervisor 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::HandoffSupervisor;

let billing = Arc::new(billing_agent);  // sets transfer_to = "refund"
let refund = Arc::new(refund_agent);
let tech = Arc::new(tech_agent);

let supervisor = HandoffSupervisor::builder("customer-service")
    .add_agent("billing", billing)
    .add_agent("refund", refund)
    .add_agent("tech", tech)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
```

## 流程

   1. 入口 agent 运行（billing）
   2. Billing 返回 `action.transfer_to = "refund"`
   3. Supervisor 把 billing 的消息推入 history
   4. Supervisor 带完整 history 运行 refund
   5. Refund 不再移交 → 给出最终答案

## 最大 handoff 次数

防止无限的 handoff 循环。默认值为 5。

## See Also
