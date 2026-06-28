# MultiAgent: Hybrid

Combine LLM-driven routing with sub-agent handoff.

## Goal

LLM picks the initial agent, sub-agents can transfer, then LLM
synthesizes the final answer.

## Use Case

Complex flows where:
- Initial routing requires LLM intelligence (intent classification)
- Sub-agents know when to escalate to specialists
- Final response needs synthesis

## Code

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::HybridSupervisor;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

let billing = Arc::new(billing_agent);   // can transfer_to = "refund"
let refund = Arc::new(refund_agent);     // final answer

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

## How It Works

1. **Initial LLM routing:** LLM picks sub-agent via tool call
2. **Sub-agent handoff:** If sub-agent returns `transfer_to`,
   supervisor forces call to target
3. **LLM synthesis:** When no more handoffs, LLM generates final
   response

## max_iterations vs max_handoffs

- `max_iterations` — Total LLM + forced handoff calls (default 10)
- `max_handoffs` — Handoff chain limit (default 5)

## See Also

- [Router how-to](./multiagent-router.md) — Pure LLM routing
- [Handoff how-to](./multiagent-handoff.md) — Pure handoff
- [MultiAgent concepts](../concepts/multiagent.md)
