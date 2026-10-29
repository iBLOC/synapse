//! A2A client wrapper — adapts the a2a-rs SDK to Synapse's streaming API.

use std::sync::Arc;

use a2a_client::{A2AClient as SdkClient, jsonrpc::JsonRpcTransport};
use futures::StreamExt;

use crate::adk::agent::AgentStreamEvent;
use crate::a2a::AgentCard;
use crate::schema::Message;

pub struct A2AClient {
    sdk_client: Arc<SdkClient<JsonRpcTransport>>,
    peer_url: String,
}

impl A2AClient {
    pub async fn new(peer_url: &str) -> Result<Self, a2a::A2AError> {
        let client = reqwest::Client::new();
        let transport = JsonRpcTransport::new(client, peer_url.to_string());
        let sdk_client = Arc::new(SdkClient::new(transport));
        Ok(Self {
            sdk_client,
            peer_url: peer_url.to_string(),
        })
    }

    /// Send a message to the remote agent and stream back events.
    pub async fn send_message_stream(
        &self,
        message: Message,
    ) -> Result<crate::stream::StreamReader<AgentStreamEvent>, a2a::A2AError> {
        let (tx, rx) = crate::stream::StreamReader::new(32);

        let sdk_client = self.sdk_client.clone();
        let peer_url = self.peer_url.clone();

        tokio::spawn(async move {
            let request = a2a::SendMessageRequest {
                message: message.into_a2a_message(),
                configuration: None,
                metadata: None,
                tenant: None,
            };

            let mut stream = match sdk_client.send_streaming_message(&request).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(AgentStreamEvent::Error {
                        agent: peer_url.clone(),
                        error: crate::adk::agent::AgentError::Agent(format!(
                            "A2A send failed: {}",
                            e
                        )),
                    }).await;
                    return;
                }
            };

            while let Some(response) = stream.next().await {
                match response {
                    Ok(stream_response) => {
                        let events = translate_a2a_response_to_agent_events(
                            stream_response,
                            &peer_url,
                        );
                        for event in events {
                            if !tx.send(event).await {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AgentStreamEvent::Error {
                            agent: peer_url.clone(),
                            error: crate::adk::agent::AgentError::Agent(format!(
                                "A2A stream error: {}",
                                e
                            )),
                        }).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Send a message and wait for a complete response (non-streaming).
    pub async fn send_message(
        &self,
        message: Message,
    ) -> Result<Message, a2a::A2AError> {
        let request = a2a::SendMessageRequest {
            message: message.into_a2a_message(),
            configuration: None,
            metadata: None,
            tenant: None,
        };

        let response = self.sdk_client.send_message(&request).await?;

        match response {
            a2a::SendMessageResponse::Task(task) => {
                task.history
                    .and_then(|h| h.into_iter().last())
                    .map(Message::from_a2a_message)
                    .ok_or_else(|| a2a::A2AError::new(-1, "No message in task"))
            }
            a2a::SendMessageResponse::Message(msg) => Ok(Message::from_a2a_message(msg)),
        }
    }

    /// Get the extended agent card of the remote peer (not yet supported in SDK).
    pub async fn get_agent_card(&self) -> Result<AgentCard, a2a::A2AError> {
        // SDK doesn't expose a plain get_agent_card; use the well-known URL path
        // via reqwest directly for now.
        let url = format!("{}/.well-known/agent-card.json", self.peer_url);
        let resp = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .map_err(|e| a2a::A2AError::new(-1, format!("HTTP error: {}", e)))?;
        let card: a2a::AgentCard = resp
            .json()
            .await
            .map_err(|e| a2a::A2AError::new(-1, format!("JSON parse error: {}", e)))?;
        Ok(AgentCard::from_a2a_card(card))
    }
}

/// Translate an A2A wire response into Synapse agent events.
///
/// `pub(crate)` so the A2A server module can round-trip test the
/// server → wire → client path in one place.
pub(crate) fn translate_a2a_response_to_agent_events(
    response: a2a::StreamResponse,
    peer_url: &str,
) -> Vec<AgentStreamEvent> {
    use a2a::StreamResponse;

    match response {
        StreamResponse::Task(_) | StreamResponse::Message(_) => Vec::new(),
        StreamResponse::ArtifactUpdate(event) => {
            let mut events = Vec::new();

            for part in &event.artifact.parts {
                match &part.content {
                    a2a::PartContent::Text(text) => {
                        events.push(AgentStreamEvent::TokenDelta {
                            agent: peer_url.to_string(),
                            delta: text.clone(),
                        });
                    }
                    a2a::PartContent::Raw(_) | a2a::PartContent::Url(_) | a2a::PartContent::Data(_) => {}
                }
            }

            if event.last_chunk.unwrap_or(false) {
                let content = event
                    .artifact
                    .parts
                    .iter()
                    .filter_map(|p| match &p.content {
                        a2a::PartContent::Text(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                events.push(AgentStreamEvent::Final {
                    agent: peer_url.to_string(),
                    message: Message::assistant(content),
                });
            }

            events
        }
        StreamResponse::StatusUpdate(event) => {
            let mut events = Vec::new();

            // A `StatusUpdate` may carry a structured tool event in its
            // metadata — see `a2a::tool_event`. The A2A wire format has
            // no tool-event variant of its own, so this side channel is
            // the only way a tool call can survive the hop. Without it
            // the tool event was flattened to text on the server side
            // and dropped here, and the downstream consumer (the
            // telencephalon end-brain) never saw a `tool_call` at all.
            if let Some(tool_event) =
                crate::a2a::tool_event::tool_event_from_metadata(event.metadata.as_ref())
            {
                events.push(crate::a2a::tool_event::tool_event_to_agent_event(
                    tool_event, peer_url,
                ));
                return events;
            }

            if event.status.state == a2a::TaskState::Failed {
                let error_msg = event
                    .status
                    .message
                    .map(|m| {
                        let schema_msg = crate::schema::Message::from_a2a_message(m);
                        match schema_msg.content {
                            crate::schema::Content::Text(s) => s,
                            _ => "Unknown error".to_string(),
                        }
                    })
                    .unwrap_or_else(|| "Unknown error".to_string());

                events.push(AgentStreamEvent::Error {
                    agent: peer_url.to_string(),
                    error: crate::adk::agent::AgentError::Agent(error_msg),
                });
            }

            events
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_update_text_parts_become_token_deltas() {
        let response = a2a::StreamResponse::ArtifactUpdate(a2a::TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "c1".to_string(),
            artifact: a2a::Artifact {
                artifact_id: String::new(),
                name: Some("response".to_string()),
                description: None,
                parts: vec![a2a::Part::text("hello")],
                metadata: None,
                extensions: None,
            },
            append: Some(true),
            last_chunk: Some(false),
            metadata: None,
        });

        let events = translate_a2a_response_to_agent_events(response, "peer");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::TokenDelta { agent, delta } => {
                assert_eq!(agent, "peer");
                assert_eq!(delta, "hello");
            }
            _ => panic!("expected TokenDelta"),
        }
    }

    #[test]
    fn artifact_last_chunk_emits_final_event() {
        let response = a2a::StreamResponse::ArtifactUpdate(a2a::TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "c1".to_string(),
            artifact: a2a::Artifact {
                artifact_id: String::new(),
                name: Some("response".to_string()),
                description: None,
                parts: vec![a2a::Part::text("done")],
                metadata: None,
                extensions: None,
            },
            append: Some(false),
            last_chunk: Some(true),
            metadata: None,
        });

        let events = translate_a2a_response_to_agent_events(response, "peer");
        // Expects: one TokenDelta (from the text part) plus one Final (last_chunk).
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], AgentStreamEvent::TokenDelta { .. }));
        match &events[1] {
            AgentStreamEvent::Final { agent, .. } => assert_eq!(agent, "peer"),
            _ => panic!("expected Final as last event"),
        }
    }

    #[test]
    fn failed_status_emits_error_event() {
        let schema_msg = crate::schema::Message::assistant("oops");
        let a2a_msg = schema_msg.into_a2a_message();

        let response = a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "c1".to_string(),
            status: a2a::TaskStatus {
                state: a2a::TaskState::Failed,
                message: Some(a2a_msg),
                timestamp: Some(chrono::Utc::now()),
            },
            metadata: None,
        });

        let events = translate_a2a_response_to_agent_events(response, "peer");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentStreamEvent::Error { agent, error } => {
                assert_eq!(agent, "peer");
                let s = format!("{}", error);
                assert!(s.contains("oops"));
            }
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn task_or_message_response_yields_no_events() {
        let task_resp = a2a::StreamResponse::Task(a2a::Task {
            id: "t".to_string(),
            context_id: "c".to_string(),
            status: a2a::TaskStatus {
                state: a2a::TaskState::Completed,
                message: None,
                timestamp: None,
            },
            history: None,
            artifacts: None,
            metadata: None,
        });
        let events = translate_a2a_response_to_agent_events(task_resp, "peer");
        assert!(events.is_empty());
    }
}
