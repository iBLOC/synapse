//! `AgentStreamEvent` — the unified event bus for agent execution.
//!
//! Every `Agent::stream_run` emits a stream of these events. Consumers
//! receive a complete picture of the agent's behavior (text deltas, tool
//! calls, internal state transitions, multi-agent routing) without needing
//! to introspect provider-specific response shapes.

use crate::adk::agent::AgentError;
use crate::schema::{Message, ToolCall};

pub type AgentStream = crate::stream::StreamReader<AgentStreamEvent>;

/// One event in an agent's execution stream.
#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    /// LLM output token delta. The consumer should append `delta` to the
    /// current assistant message in their UI / aggregator.
    TokenDelta { agent: String, delta: String },

    /// A complete, well-formed tool call has been assembled from
    /// [`Self::ToolStart`] / token deltas. Carries the fully-built call.
    ToolCall { agent: String, call: ToolCall },

    /// Tool execution start (pre-middleware). `args` is the raw JSON string.
    ToolStart { agent: String, name: String, args: String },

    /// Tool execution end (post-middleware). `latency_ms` is wall-clock time
    /// spent in the tool implementation, excluding middleware.
    ToolEnd {
        agent: String,
        name: String,
        result: String,
        latency_ms: u64,
    },

    /// Internal state transition (ReAct iteration, Graph node entered/exited).
    StepTransition {
        agent: String,
        from: String,
        to: String,
        step: usize,
    },

    /// Multi-agent routing decision (e.g. `RouterSupervisor` picks a sub-agent).
    RouteDecision { supervisor: String, target: String, reason: String },

    /// Multi-agent handoff (e.g. `HandoffSupervisor` transfers conversation).
    Handoff { from: String, to: String, context_size: usize },

    /// Successful terminal event. No more events will be emitted for this run.
    Final { agent: String, message: Message },

    /// Failure terminal event. No more events will be emitted for this run.
    Error { agent: String, error: AgentError },
}

impl AgentStreamEvent {
    /// Returns the agent name carried by this event, or `None` for events
    /// that don't carry one.
    pub fn agent_name(&self) -> Option<&str> {
        match self {
            Self::TokenDelta { agent, .. }
            | Self::ToolCall { agent, .. }
            | Self::ToolStart { agent, .. }
            | Self::ToolEnd { agent, .. }
            | Self::StepTransition { agent, .. }
            | Self::Final { agent, .. }
            | Self::Error { agent, .. } => Some(agent.as_str()),
            Self::RouteDecision { supervisor, .. } => Some(supervisor.as_str()),
            Self::Handoff { from, .. } => Some(from.as_str()),
        }
    }

    /// Returns `true` for terminal events ([`Self::Final`] and [`Self::Error`]).
    /// Consumers that care about stream-end use this to detect completion
    /// without relying on channel closure.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Final { .. } | Self::Error { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_terminal_only_for_final_and_error() {
        let e = AgentStreamEvent::Error {
            agent: "x".into(),
            error: AgentError::Interrupted,
        };
        assert!(e.is_terminal());

        let f = AgentStreamEvent::Final {
            agent: "x".into(),
            message: Message::user(""),
        };
        assert!(f.is_terminal());

        let t = AgentStreamEvent::TokenDelta {
            agent: "x".into(),
            delta: "hi".into(),
        };
        assert!(!t.is_terminal());
    }

    #[test]
    fn agent_name_extraction() {
        let e = AgentStreamEvent::Handoff {
            from: "a".into(),
            to: "b".into(),
            context_size: 1,
        };
        assert_eq!(e.agent_name(), Some("a"));
    }
}
