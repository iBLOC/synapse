//! A2A client wrapper — adapts the a2a-rs SDK to Synapse's streaming API.

use std::sync::Arc;

use a2a_client::{A2AClient as SdkClient, jsonrpc::JsonRpcTransport};
use futures::StreamExt;

use crate::adk::agent::AgentStreamEvent;
use crate::a2a::AgentCard;
use crate::schema::Message;


/// Client-side interceptor that requests A2A extensions on every call.
///
/// The SDK turns each service param into an HTTP header, so putting the
/// extension URIs into the params is what actually puts them on the wire.
///
/// Both header spellings are sent: `A2A-Extensions` (A2A v1.0) and
/// `X-A2A-Extensions` (the name ADK still uses). A peer that only knows
/// one of them still sees the request.
struct ExtensionRequestInterceptor {
    uris: Vec<String>,
}

impl ExtensionRequestInterceptor {
    fn new(uris: Vec<String>) -> Self {
        Self { uris }
    }

    /// The comma-joined header value.
    fn header_value(&self) -> String {
        self.uris.join(",")
    }
}

#[async_trait::async_trait]
impl a2a_client::middleware::CallInterceptor for ExtensionRequestInterceptor {
    async fn before(
        &self,
        _method: &str,
        params: &mut a2a_client::ServiceParams,
    ) -> Result<(), a2a::A2AError> {
        if self.uris.is_empty() {
            return Ok(());
        }
        let value = self.header_value();
        params.insert(
            crate::a2a::extension::HEADER_A2A_EXTENSIONS.to_string(),
            vec![value.clone()],
        );
        params.insert(
            crate::a2a::extension::HEADER_X_A2A_EXTENSIONS.to_string(),
            vec![value],
        );
        Ok(())
    }
}
pub struct A2AClient {
    sdk_client: Arc<SdkClient<JsonRpcTransport>>,
    peer_url: String,
}

impl A2AClient {
    pub async fn new(peer_url: &str) -> Result<Self, a2a::A2AError> {
        Self::with_extensions(peer_url, Vec::new()).await
    }

    /// Build a client that requests the given extensions on every call.
    ///
    /// The extension URIs are attached as request headers by an
    /// interceptor, so they ride along with every method -- streaming
    /// sends, task fetches and cancels alike -- rather than only with the
    /// first one.
    pub async fn with_extensions(
        peer_url: &str,
        extensions: Vec<String>,
    ) -> Result<Self, a2a::A2AError> {
        let client = peer_http_client(peer_url);
        let transport = JsonRpcTransport::new(client, peer_url.to_string());
        let mut sdk_client = SdkClient::new(transport);
        if !extensions.is_empty() {
            sdk_client = sdk_client.with_interceptors(vec![Arc::new(
                ExtensionRequestInterceptor::new(extensions),
            )]);
        }
        Ok(Self {
            sdk_client: Arc::new(sdk_client),
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

            let mut progress = StreamProgress::default();

            while let Some(response) = stream.next().await {
                match response {
                    Ok(stream_response) => {
                        let events = translate_a2a_response_to_agent_events(
                            stream_response,
                            &peer_url,
                            &mut progress,
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

            // ⚠⚠ **流结束了 ≠ 交互结束了** ——
            //
            // A2A v1.0：任务只有在收到一个**终态**事件时才算结束。流自己
            // 断了（对端崩了／网断了／代理掐了）而终态没到 ⇒ ⇒ 这次交互是
            // **被截断**的。Google ADK 的客户端正是这么读的
            // （`_UNFINISHED_TASK_STATES` ＋ 逐字「A2A response stream
            // ended before task ... reached a terminal state」）⇒ ⇒ 静默
            // 收尾等于把「不完整」当「完成」报出去。
            if !progress.saw_final && !progress.saw_error {
                if !progress.saw_terminal {
                    let _ = tx.send(AgentStreamEvent::Error {
                        agent: peer_url.clone(),
                        error: crate::adk::agent::AgentError::Agent(
                            "the peer's stream ended before the task reached a terminal state"
                                .to_string(),
                        ),
                    }).await;
                } else if !progress.saw_artifact_text {
                    // 终态到了、答复却不在 artifact 里 ⇒ ⇒ 那它就在
                    // status message 里（有些对端**只**放那儿）。
                    if let Some(text) = progress.status_text.clone() {
                        let _ = tx.send(AgentStreamEvent::Final {
                            agent: peer_url.clone(),
                            message: Message::assistant(text),
                        }).await;
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
            a2a::SendMessageResponse::Task(task) => task_reply_from_task(&task),
            a2a::SendMessageResponse::Message(msg) => Ok(Message::from_a2a_message(msg)),
        }
    }

    /// Get the extended agent card of the remote peer (not yet supported in SDK).
    pub async fn get_agent_card(&self) -> Result<AgentCard, a2a::A2AError> {
        // SDK doesn't expose a plain get_agent_card; use the well-known URL path
        // via reqwest directly for now.
        let url = format!("{}/.well-known/agent-card.json", self.peer_url);
        let resp = peer_http_client(&self.peer_url)
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

    /// Send a message and return the raw SDK response.
    ///
    /// The standard `message/send` answers either with a `Task` (stateful
    /// agents) or with a plain `Message` (stateless ones). The convenience
    /// `send_message` above collapses both into a `Message`, which drops
    /// the peer's task id — callers that want to poll or cancel that task
    /// later need this variant.
    pub async fn send_message_response(
        &self,
        message: Message,
    ) -> Result<a2a::SendMessageResponse, a2a::A2AError> {
        let request = a2a::SendMessageRequest {
            message: message.into_a2a_message(),
            configuration: None,
            metadata: None,
            tenant: None,
        };

        self.sdk_client.send_message(&request).await
    }

    /// Fetch a peer's task by id (`tasks/get`).
    pub async fn get_task(&self, task_id: &str) -> Result<a2a::Task, a2a::A2AError> {
        let request = a2a::GetTaskRequest {
            id: task_id.to_string(),
            history_length: None,
            tenant: None,
        };

        self.sdk_client.get_task(&request).await
    }

    /// Cancel a peer's task by id (`tasks/cancel`).
    pub async fn cancel_task(&self, task_id: &str) -> Result<a2a::Task, a2a::A2AError> {
        let request = a2a::CancelTaskRequest {
            id: task_id.to_string(),
            metadata: None,
            tenant: None,
        };

        self.sdk_client.cancel_task(&request).await
    }
}


/// The HTTP client used to talk to a peer.
///
/// ⚠⚠ A loopback peer must not go through the system proxy -- see
/// [`crate::http_client`] for the whole story (and the measurement).
fn peer_http_client(peer_url: &str) -> reqwest::Client {
    crate::http_client::client_for(peer_url)
}
/// The peer's answer to a task -- taken the way the official A2A client
/// takes it.
///
/// The reference is Google ADK's task converter
/// (`google/adk/a2a/converters/to_adk_event.py`):
///
/// - the answer lives in `task.artifacts`;
/// - `status.message` is a reply source only in the boundary states
///   (`input_required` / `auth_required`, or a boundary state that
///   carries no artifacts at all);
/// - a **failed** task's `status.message` is the *error*, not an answer
///   -- ADK stamps the event with an error code and message instead of
///   handing that text back as content.
///
/// `task.history` is deliberately not the primary source. A peer that
/// echoes the caller's own turn into history (synapse's own server does
/// exactly that) would otherwise hand the caller's own words back as the
/// agent's answer. History is only consulted as a last resort, and only
/// for a turn the **agent** produced (`role = agent`).
pub(crate) fn task_reply_from_task(task: &a2a::Task) -> Result<Message, a2a::A2AError> {
    let state = task.status.state.clone();
    let status_text = status_message_text(task.status.message.as_ref());

    // ⚠⚠ A failure must not look like an answer.
    //
    // Reporting the peer's error text as the reply is the "failure looks
    // like success" shape: the caller gets a perfectly well-formed
    // message that happens to say "401 UNAUTHENTICATED".
    match state {
        a2a::TaskState::Failed => {
            return Err(a2a::A2AError::new(
                -1,
                status_text.unwrap_or_else(|| "the peer's task failed".to_string()),
            ));
        }
        a2a::TaskState::Canceled => {
            return Err(a2a::A2AError::new(
                -1,
                status_text.unwrap_or_else(|| "the peer's task was canceled".to_string()),
            ));
        }
        a2a::TaskState::Rejected => {
            return Err(a2a::A2AError::new(
                -1,
                status_text.unwrap_or_else(|| "the peer's task was rejected".to_string()),
            ));
        }
        _ => {}
    }

    // The answer is in the artifacts.
    let artifact_parts: Vec<a2a::Part> = task
        .artifacts
        .as_ref()
        .map(|artifacts| {
            artifacts
                .iter()
                .flat_map(|artifact| artifact.parts.iter().cloned())
                .collect()
        })
        .unwrap_or_default();
    if let Some(reply) = reply_from_parts(&artifact_parts) {
        return Ok(reply);
    }

    // No artifact: the status message is the reply. That is how a peer
    // asks for input or for auth.
    if let Some(text) = status_text {
        return Ok(Message::assistant(text));
    }

    // Last resort: the agent's own turn in the history -- *not* the last
    // history entry, which may be the caller's own message.
    if let Some(history) = task.history.as_ref()
        && let Some(turn) = history.iter().rev().find(|m| m.role == a2a::Role::Agent)
        && let Some(reply) = reply_from_parts(&turn.parts)
    {
        return Ok(reply);
    }

    Err(a2a::A2AError::new(
        -1,
        format!("the peer's task carries no reply (state: {:?})", state),
    ))
}

/// The text of a status message, if it has any.
fn status_message_text(message: Option<&a2a::Message>) -> Option<String> {
    let message = message?;
    let text = message
        .parts
        .iter()
        .filter_map(|part| match &part.content {
            a2a::PartContent::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() { None } else { Some(text.to_string()) }
}

/// Build the reply from a peer's parts.
///
/// Text parts are concatenated with no separator, matching the way this
/// module already joins the parts of a streaming artifact into a final
/// message. Parts that carry something else (an image, a data blob) are
/// rendered by the SDK's own conversion, so they are not silently
/// dropped.
fn reply_from_parts(parts: &[a2a::Part]) -> Option<Message> {
    if parts.is_empty() {
        return None;
    }

    let text = parts
        .iter()
        .filter_map(|part| match &part.content {
            a2a::PartContent::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    if !text.is_empty() {
        return Some(Message::assistant(text));
    }

    let rendered = Message::from_a2a_message(a2a::Message {
        message_id: format!("msg-{}", uuid::Uuid::new_v4()),
        context_id: None,
        task_id: None,
        role: a2a::Role::Agent,
        parts: parts.to_vec(),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    });
    let empty = match &rendered.content {
        crate::schema::Content::Text(text) => text.is_empty(),
        _ => true,
    };
    if empty { None } else { Some(rendered) }
}

/// What a peer's stream has delivered so far.
///
/// The A2A wire format lets the answer arrive in two places: an
/// artifact (the usual one) or the status message of a status update.
/// Google ADK puts it in **both** — reading both would show the answer
/// twice — so the artifact wins, and the status message is kept only as
/// a fallback for peers that deliver the answer *only* that way.
#[derive(Debug, Default)]
pub(crate) struct StreamProgress {
    /// Any artifact text was seen.
    pub saw_artifact_text: bool,
    /// A final answer was emitted.
    pub saw_final: bool,
    /// An error was emitted.
    pub saw_error: bool,
    /// The peer reported a state that closes the interaction.
    pub saw_terminal: bool,
    /// The last status message that was not an error.
    pub status_text: Option<String>,
    /// The text of the artifacts seen so far, per artifact id.
    ///
    /// A peer may stream the answer in chunks: the first update creates
    /// the artifact (`append = false`), the later ones append to it
    /// (`append = true`), and the last one closes it (`last_chunk =
    /// true`). The final message must carry **all** of it, not just the
    /// last chunk.
    pub artifacts: std::collections::HashMap<String, String>,
}

/// Whether a task state closes the current interaction.
///
/// `TaskState::is_terminal` covers completed / failed / canceled /
/// rejected. `input_required` and `auth_required` are *not* terminal —
/// the task stays open — but they do end the current turn: the peer is
/// asking for something.
fn closes_the_interaction(state: &a2a::TaskState) -> bool {
    state.is_terminal()
        || matches!(
            state,
            a2a::TaskState::InputRequired | a2a::TaskState::AuthRequired
        )
}

/// Translate an A2A wire response into Synapse agent events.
///
/// `pub(crate)` so the A2A server module can round-trip test the
/// server → wire → client path in one place.
///
/// `progress` records what the stream has delivered so far; the caller
/// uses it to decide how the stream ended (see `send_message_stream`).
pub(crate) fn translate_a2a_response_to_agent_events(
    response: a2a::StreamResponse,
    peer_url: &str,
    progress: &mut StreamProgress,
) -> Vec<AgentStreamEvent> {
    use a2a::StreamResponse;

    match response {
        StreamResponse::Task(task) => {
            if closes_the_interaction(&task.status.state) {
                progress.saw_terminal = true;
            }
            Vec::new()
        }
        StreamResponse::Message(_) => Vec::new(),
        StreamResponse::ArtifactUpdate(event) => {
            let mut events = Vec::new();

            for part in &event.artifact.parts {
                match &part.content {
                    a2a::PartContent::Text(text) => {
                        progress.saw_artifact_text = true;
                        events.push(AgentStreamEvent::TokenDelta {
                            agent: peer_url.to_string(),
                            delta: text.clone(),
                        });
                    }
                    a2a::PartContent::Raw(_) | a2a::PartContent::Url(_) | a2a::PartContent::Data(_) => {}
                }
            }

            // ⚠⚠ **答复是整个 artifact，不是最后一块** ——
            //
            // 对端可以分块流式送答复：第一个更新**建** artifact
            // （append=false），之后的**续**（append=true），最后一块
            // （last_chunk=true）收尾 ⇒ ⇒ 只取最后一块 ⇒ ⇒ 答复被截断
            // ⇒ ⇒ Final 必须带累加出来的全文。
            let chunk = event
                .artifact
                .parts
                .iter()
                .filter_map(|p| match &p.content {
                    a2a::PartContent::Text(s) => Some(s.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
            let accumulated = progress
                .artifacts
                .entry(event.artifact.artifact_id.clone())
                .or_default();
            if !event.append.unwrap_or(false) {
                // `append = false` means "replace the artifact".
                accumulated.clear();
            }
            accumulated.push_str(&chunk);
            let content = accumulated.clone();

            if event.last_chunk.unwrap_or(false) {
                progress.saw_final = true;
                events.push(AgentStreamEvent::Final {
                    agent: peer_url.to_string(),
                    message: Message::assistant(content),
                });
            }

            events
        }
        StreamResponse::StatusUpdate(event) => {
            let mut events = Vec::new();

            // The state is read **first**: the tool-event branch below
            // returns early, and the state must not be lost with it —
            // it is what tells the caller the interaction is over.
            if closes_the_interaction(&event.status.state) {
                progress.saw_terminal = true;
            }

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

                progress.saw_error = true;
                events.push(AgentStreamEvent::Error {
                    agent: peer_url.to_string(),
                    error: crate::adk::agent::AgentError::Agent(error_msg),
                });
            } else if let Some(text) = status_message_text(event.status.message.as_ref()) {
                // A status message that is not an error may still carry
                // the answer: some peers deliver it **only** this way.
                progress.status_text = Some(text);
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

        let events = translate_a2a_response_to_agent_events(response, "peer", &mut StreamProgress::default());
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

        let events = translate_a2a_response_to_agent_events(response, "peer", &mut StreamProgress::default());
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

        let events = translate_a2a_response_to_agent_events(response, "peer", &mut StreamProgress::default());
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
        let events = translate_a2a_response_to_agent_events(task_resp, "peer", &mut StreamProgress::default());
        assert!(events.is_empty());
    }
}
