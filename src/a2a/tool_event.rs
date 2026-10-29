//! Structured tool-event channel for the A2A v0.2 wire.
//!
//! # Why this module exists
//!
//! The official A2A `StreamResponse` enum has exactly four variants —
//! `Task`, `Message`, `StatusUpdate`, `ArtifactUpdate` — and **none of
//! them can carry a tool call**. The A2A server therefore used to
//! *flatten* `AgentStreamEvent::ToolStart` into a human-readable status
//! message (`"Calling tool: name(args)"`), and the A2A client used to
//! drop every `StatusUpdate` that wasn't a failure.
//!
//! Net effect: a tool call crossing an A2A hop was **destroyed on the
//! wire**. The downstream consumer (the telencephalon end-brain, for
//! one) could only ever see text, so its `tool_call` branch had no
//! reachable producer at all — the events existed in the type system and
//! in the translation tables, but nothing could put them on the wire.
//!
//! # The fix
//!
//! `StatusUpdate` does have a free-form `metadata` map, so we use it as
//! a structured side channel: the server writes a [`ToolEvent`] under
//! [`TOOL_EVENT_KEY`], and the client reads it back and rebuilds the
//! original [`AgentStreamEvent`].
//!
//! The human-readable text is **still emitted alongside** the metadata,
//! so peers that predate this change keep seeing exactly what they saw
//! before. Nothing on the wire is removed — a side channel is added.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adk::agent::AgentStreamEvent;
use crate::schema::ToolCall;

/// Metadata key under which a `StatusUpdate` carries a structured tool
/// event.
pub const TOOL_EVENT_KEY: &str = "synapse_tool_event";

/// The structured tool event carried inside `StatusUpdate.metadata`.
///
/// Wire shape (camelCase-free, `snake_case` tag):
///
/// ```json
/// { "kind": "start", "name": "set_theme", "args": "{\"color\":\"#71717A\"}" }
/// { "kind": "call",   "call": { "id": "...", "tool_type": "function",
///                              "function": { "name": "...", "arguments": "..." } } }
/// { "kind": "end",    "name": "set_theme", "result": "ok", "latency_ms": 12 }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolEvent {
    /// Tool execution started. `args` is the raw JSON argument string.
    Start { name: String, args: String },
    /// A fully-assembled tool call.
    Call { call: ToolCall },
    /// Tool execution finished.
    End {
        name: String,
        result: String,
        latency_ms: u64,
    },
}

/// Extract the structured tool event from an [`AgentStreamEvent`], if it
/// is one.
///
/// Returns `None` for every event that isn't a tool event — the caller
/// then falls back to the normal (text-only) translation.
pub fn tool_event_of(event: &AgentStreamEvent) -> Option<ToolEvent> {
    match event {
        AgentStreamEvent::ToolStart { name, args, .. } => Some(ToolEvent::Start {
            name: name.clone(),
            args: args.clone(),
        }),
        AgentStreamEvent::ToolCall { call, .. } => Some(ToolEvent::Call { call: call.clone() }),
        AgentStreamEvent::ToolEnd {
            name,
            result,
            latency_ms,
            ..
        } => Some(ToolEvent::End {
            name: name.clone(),
            result: result.clone(),
            latency_ms: *latency_ms,
        }),
        _ => None,
    }
}

/// Serialize a [`ToolEvent`] into the value that goes into
/// `StatusUpdate.metadata`.
pub fn tool_event_to_value(event: &ToolEvent) -> Value {
    // `ToolEvent` is a plain owned-data enum; serialization cannot fail.
    serde_json::to_value(event).unwrap_or(Value::Null)
}

/// Read a [`ToolEvent`] back out of a `StatusUpdate.metadata` map.
///
/// Returns `None` when the map is absent, when it has no
/// [`TOOL_EVENT_KEY`] entry, or when the entry doesn't deserialize —
/// a malformed side channel must never break the whole stream.
pub fn tool_event_from_metadata(metadata: Option<&HashMap<String, Value>>) -> Option<ToolEvent> {
    let raw = metadata?.get(TOOL_EVENT_KEY)?;
    serde_json::from_value(raw.clone()).ok()
}

/// Rebuild the original [`AgentStreamEvent`] from a decoded
/// [`ToolEvent`].
///
/// `agent` is the peer URL: on the wire the event has no agent of its
/// own, so the receiving side stamps the peer it came from.
pub fn tool_event_to_agent_event(event: ToolEvent, agent: &str) -> AgentStreamEvent {
    match event {
        ToolEvent::Start { name, args } => AgentStreamEvent::ToolStart {
            agent: agent.to_string(),
            name,
            args,
        },
        ToolEvent::Call { call } => AgentStreamEvent::ToolCall {
            agent: agent.to_string(),
            call,
        },
        ToolEvent::End {
            name,
            result,
            latency_ms,
        } => AgentStreamEvent::ToolEnd {
            agent: agent.to_string(),
            name,
            result,
            latency_ms,
        },
    }
}

/// Build the `metadata` map for a `StatusUpdate` that carries a tool
/// event.
///
/// Returns `None` when the event isn't a tool event.
pub fn tool_event_metadata(event: &AgentStreamEvent) -> Option<HashMap<String, Value>> {
    let te = tool_event_of(event)?;
    let mut metadata = HashMap::new();
    metadata.insert(TOOL_EVENT_KEY.to_string(), tool_event_to_value(&te));
    Some(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FunctionCall;

    fn call() -> ToolCall {
        ToolCall {
            id: "call-1".into(),
            tool_type: "function".into(),
            function: FunctionCall {
                name: "set_theme".into(),
                arguments: "{\"color\":\"#71717A\"}".into(),
            },
        }
    }

    #[test]
    fn tool_start_round_trips_through_metadata() {
        let event = AgentStreamEvent::ToolStart {
            agent: "peer".into(),
            name: "set_theme".into(),
            args: "{\"color\":\"#71717A\"}".into(),
        };
        let metadata = tool_event_metadata(&event).expect("tool start must carry metadata");
        let decoded = tool_event_from_metadata(Some(&metadata)).expect("must decode");
        let rebuilt = tool_event_to_agent_event(decoded, "peer");
        match rebuilt {
            AgentStreamEvent::ToolStart { name, args, agent } => {
                assert_eq!(agent, "peer");
                assert_eq!(name, "set_theme");
                assert_eq!(args, "{\"color\":\"#71717A\"}");
            }
            other => panic!("expected ToolStart, got {other:?}"),
        }
    }

    #[test]
    fn tool_call_round_trips_through_metadata() {
        let event = AgentStreamEvent::ToolCall {
            agent: "peer".into(),
            call: call(),
        };
        let metadata = tool_event_metadata(&event).expect("tool call must carry metadata");
        let decoded = tool_event_from_metadata(Some(&metadata)).expect("must decode");
        let rebuilt = tool_event_to_agent_event(decoded, "peer");
        match rebuilt {
            AgentStreamEvent::ToolCall { call: c, .. } => {
                assert_eq!(c.id, "call-1");
                assert_eq!(c.function.name, "set_theme");
                assert_eq!(c.function.arguments, "{\"color\":\"#71717A\"}");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn tool_end_round_trips_through_metadata() {
        let event = AgentStreamEvent::ToolEnd {
            agent: "peer".into(),
            name: "set_theme".into(),
            result: "ok".into(),
            latency_ms: 42,
        };
        let metadata = tool_event_metadata(&event).expect("tool end must carry metadata");
        let decoded = tool_event_from_metadata(Some(&metadata)).expect("must decode");
        let rebuilt = tool_event_to_agent_event(decoded, "peer");
        match rebuilt {
            AgentStreamEvent::ToolEnd {
                name,
                result,
                latency_ms,
                ..
            } => {
                assert_eq!(name, "set_theme");
                assert_eq!(result, "ok");
                assert_eq!(latency_ms, 42);
            }
            other => panic!("expected ToolEnd, got {other:?}"),
        }
    }

    #[test]
    fn non_tool_events_carry_no_metadata() {
        let event = AgentStreamEvent::TokenDelta {
            agent: "peer".into(),
            delta: "hi".into(),
        };
        assert!(tool_event_metadata(&event).is_none());
        assert!(tool_event_of(&event).is_none());
    }

    #[test]
    fn missing_or_malformed_metadata_decodes_to_none() {
        assert!(tool_event_from_metadata(None).is_none());
        let empty: HashMap<String, Value> = HashMap::new();
        assert!(tool_event_from_metadata(Some(&empty)).is_none());

        let mut wrong = HashMap::new();
        wrong.insert(TOOL_EVENT_KEY.to_string(), Value::String("nope".into()));
        assert!(tool_event_from_metadata(Some(&wrong)).is_none());
    }

    #[test]
    fn wire_shape_uses_snake_case_kind_tag() {
        let event = AgentStreamEvent::ToolStart {
            agent: "peer".into(),
            name: "n".into(),
            args: "{}".into(),
        };
        let metadata = tool_event_metadata(&event).unwrap();
        let raw = &metadata[TOOL_EVENT_KEY];
        assert_eq!(raw["kind"], "start");
        assert_eq!(raw["name"], "n");
        assert_eq!(raw["args"], "{}");
    }
}
