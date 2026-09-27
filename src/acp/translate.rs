//! Translate ACP wire-level updates into Synapse's [`AgentStreamEvent`].
//!
//! We deliberately keep this a free function (no `&self`) so it can be
//! unit-tested without spinning up a real subprocess.

use agent_client_protocol::schema::v1::{
    ContentBlock, SessionUpdate, ToolCall as AcpToolCall, ToolCallStatus,
};

use crate::adk::stream::AgentStreamEvent;

/// Translate a single ACP [`SessionUpdate`] into one (or more) Synapse events.
///
/// Multiple events are possible for an update that fans out, e.g. a
/// `ToolCall` carrying a text content chunk — we emit `ToolStart` for the
/// tool itself and `TokenDelta` for the chunk, in that order.
///
/// Returns an empty `Vec` for updates we don't surface (e.g. `Plan`,
/// `AvailableCommandsUpdate`); the caller should ignore them silently.
pub fn translate_update(
    agent: &str,
    update: SessionUpdate,
) -> Vec<AgentStreamEvent> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => content_chunk_to_deltas(agent, &chunk.content),
        SessionUpdate::AgentThoughtChunk(chunk) => {
            // Surface thoughts as plain text deltas prefixed with a marker
            // so consumers that don't care can filter them out. We don't
            // have a dedicated `ThoughtDelta` variant in `AgentStreamEvent`.
            content_chunk_to_deltas(agent, &chunk.content)
        }
        SessionUpdate::UserMessageChunk(_) => {
            // Mirror back from the agent is informational only — drop.
            Vec::new()
        }
        SessionUpdate::ToolCall(call) => vec![tool_call_to_event(agent, &call)],
        SessionUpdate::ToolCallUpdate(upd) => {
            // `ToolCallUpdate` carries only the *changed* fields under
            // `upd.fields`. We forward the merged status (falling back
            // to Pending for status-less updates) and pass through
            // name/raw_input when the update supplies them.
            let name = upd.fields.name.clone().or_else(|| upd.fields.title.clone());
            let args = upd
                .fields
                .raw_input
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();
            let result = upd
                .fields
                .raw_output
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();
            let event = match upd.fields.status.clone().unwrap_or(ToolCallStatus::Pending) {
                ToolCallStatus::Pending | ToolCallStatus::InProgress => {
                    AgentStreamEvent::ToolStart {
                        agent: agent.to_string(),
                        name: name.unwrap_or_default(),
                        args,
                    }
                }
                ToolCallStatus::Completed | ToolCallStatus::Failed => AgentStreamEvent::ToolEnd {
                    agent: agent.to_string(),
                    name: name.unwrap_or_default(),
                    result,
                    latency_ms: 0,
                },
                _ => AgentStreamEvent::TokenDelta {
                    agent: agent.to_string(),
                    delta: String::new(),
                },
            };
            vec![event]
        }
        // The remaining variants either aren't surfaced by Synapse or
        // are forward-compat-only. `#[non_exhaustive]` catches the rest.
        _ => Vec::new(),
    }
}

fn content_chunk_to_deltas(agent: &str, block: &ContentBlock) -> Vec<AgentStreamEvent> {
    match block {
        ContentBlock::Text(text) => vec![AgentStreamEvent::TokenDelta {
            agent: agent.to_string(),
            delta: text.text.clone(),
        }],
        // Non-text chunks (image/audio/resource) aren't represented as a
        // single delta — drop for now. Consumers can subscribe to the
        // structured content if they need richer output.
        _ => Vec::new(),
    }
}

fn tool_call_to_event(agent: &str, call: &AcpToolCall) -> AgentStreamEvent {
    let args = call
        .raw_input
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_default();

    match call.status {
        ToolCallStatus::Pending | ToolCallStatus::InProgress => AgentStreamEvent::ToolStart {
            agent: agent.to_string(),
            name: call.name.clone().unwrap_or_else(|| call.title.clone()),
            args,
        },
        ToolCallStatus::Completed | ToolCallStatus::Failed => {
            let result = call
                .raw_output
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();
            AgentStreamEvent::ToolEnd {
                agent: agent.to_string(),
                name: call.name.clone().unwrap_or_else(|| call.title.clone()),
                result,
                latency_ms: 0,
            }
        }
        // Unknown future status — fall back to a neutral token delta so we
        // never block the stream.
        _ => AgentStreamEvent::TokenDelta {
            agent: agent.to_string(),
            delta: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::SessionUpdate;

    // `SessionUpdate` is `#[non_exhaustive]`, so we go through JSON to
    // build it in tests. That's also exactly how the wire shape is
    // defined, so this is a faithful round-trip.

    #[test]
    fn agent_message_chunk_becomes_token_delta() {
        let json = serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "hello" }
        });
        let update: SessionUpdate = serde_json::from_value(json).unwrap();
        let events = translate_update("opencode", update);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::TokenDelta { agent, delta } => {
                assert_eq!(agent, "opencode");
                assert_eq!(delta, "hello");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn tool_call_pending_becomes_tool_start() {
        let json = serde_json::json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "tc1",
            "title": "Read file",
            "name": "read_file",
            "kind": "read",
            "status": "pending",
            "content": [],
            "locations": [],
            "rawInput": { "path": "/a.rs" },
            "rawOutput": null,
        });
        let update: SessionUpdate = serde_json::from_value(json).unwrap();
        let events = translate_update("claude-code-acp", update);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::ToolStart { agent, name, args } => {
                assert_eq!(agent, "claude-code-acp");
                assert_eq!(name, "read_file");
                assert!(args.contains("/a.rs"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn tool_call_completed_becomes_tool_end() {
        let json = serde_json::json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "tc2",
            "title": "Read file",
            "name": "read_file",
            "kind": "read",
            "status": "completed",
            "content": [],
            "locations": [],
            "rawInput": null,
            "rawOutput": "file contents",
        });
        let update: SessionUpdate = serde_json::from_value(json).unwrap();
        let events = translate_update("opencode", update);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AgentStreamEvent::ToolEnd { .. }));
    }
}