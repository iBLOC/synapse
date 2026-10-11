---
title: MultiAgent Hybrid
---

# MultiAgent：混合模式

把 LLM 驱动的 routing 与 sub-agent handoff 结合起来。

## 目标

由 LLM 选出初始 agent，sub-agent 可以移交，最后再由 LLM 综合出最终答案。

## 使用场景

适用于满足以下条件的复杂流程：

  - 初始 routing 需要 LLM 智能（意图分类）
  - sub-agent 知道何时升级给专家
  - 最终响应需要综合

## 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::HybridSupervisor;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

let billing = Arc::new(billing_agent);
let refund = Arc::new(refund_agent);

let hybrid = HybridSupervisor::builder("hybrid", model)
    .description("LLM routes, sub-agents can transfer")
    .add_agent("billing", billing)
    .add_agent("refund", refund)
    .max_iterations(10)
    .max_handoffs(5)
    .build();

let event = hybrid.run(AgentInput {
    messages: vec![Message::user("I want my money back")],
    enable_streaming: false,
}).await?;

// Flow:
// 1. LLM picks billing (via tool call)
// 2. Billing runs, returns transfer_to = "refund"
// 3. Supervisor forces call to refund (no LLM consult)
// 4. Refund runs, no transfer
// 5. LLM synthesizes final answer
```

## 工作原理

   1. 初始 LLM routing：LLM 通过 tool call 选出 sub-agent
   sub-agent handoff：如果 sub-agent 返回 `transfer_to`，supervisor 强制调用目标 agent
   LLM synthesis：没有更多 handoff 时，由 LLM 生成最终响应

## See Also
