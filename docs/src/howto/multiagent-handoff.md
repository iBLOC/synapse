# MultiAgent: Conversational Handoff

Sub-agents decide when to transfer the conversation to another agent.

## Goal

Build a multi-role system where agents can hand off conversations
dynamically based on context.

## Use Case

- Customer service (billing → refund → tech support)
- Multi-expert consultation (GP → specialist → surgeon)
- Help desk triage (L1 → L2 → L3)

## How Sub-Agents Signal Handoff

A sub-agent signals handoff by setting `event.action.transfer_to`:

```rust
async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
    // Detect that this is a refund question, not billing
    let task = extract_task(&input);

    Ok(AgentEvent {
        agent_name: self.name.clone(),
        output: AgentOutput {
            message: Some(Message::assistant(
                "This appears to be a refund question, transferring to refund agent"
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

## Supervisor Code

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

let event = supervisor.run(AgentInput {
    messages: vec![Message::user("I want my money back")],
    enable_streaming: false,
}).await?;

// Flow: billing → (transfer_to: "refund") → refund → final answer
```

## Flow

1. Entry agent runs (billing)
2. Billing returns `action.transfer_to = "refund"`
3. Supervisor pushes billing's message to history
4. Supervisor runs refund with full history
5. Refund returns no transfer → final answer

## max_handoffs

Prevents infinite handoff loops. Default: 5.

```rust
.max_handoffs(10)  // allow up to 10 handoffs
```

## No LLM Required

`HandoffSupervisor` is **not** an LLM agent — it's a simple
coordinator. Sub-agents themselves can be LLMs, but the
orchestration is rule-based.

## When to Use

- Multi-role conversation (entry point + dynamic routing)
- When sub-agents have better context about when to transfer than a
  top-level LLM
- Customer service flows with clear escalation paths

## See Also

- [Hybrid how-to](./multiagent-hybrid.md) — LLM routing + handoff
- [MultiAgent concepts](../concepts/multiagent.md)
