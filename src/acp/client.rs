//! Owns the connection to one ACP agent subprocess.
//!
//! [`AcpClient::connect`] spawns the agent, performs the `initialize`
//! handshake, then starts a long-lived tokio task that processes
//! prompt requests serially. Each [`AcpSessionHandle::prompt`] opens
//! a fresh ACP session (`session/new`), runs the prompt, and streams
//! the translated [`AgentStreamEvent`]s back to the caller as a
//! [`StreamReader`].
//!
//! Why a background task: the SDK's `Client::connect_with` body
//! *blocks* the dispatch loop, so all `send_request` calls must happen
//! inside its `main_fn` closure. We can't hold a `ConnectionTo<Agent>`
//! across awaits from user code. Instead we own the connection on a
//! dedicated task and route prompt requests to it over an mpsc channel.

use std::sync::Arc;

use agent_client_protocol::schema::v1::{InitializeRequest, NewSessionRequest};
use agent_client_protocol::{
    AcpAgent, Agent as AcpAgentRole, Client, ConnectionTo, schema::ProtocolVersion,
};
use tokio::sync::mpsc;

use super::session::PromptRequest;
use super::spec::AcpAgentSpec;
use super::translate::translate_update;
use crate::adk::stream::AgentStreamEvent;
use crate::stream::StreamReader;

/// A connected client to one external ACP agent.
///
/// Cheap to clone (the inner state is `Arc`-shared); drop the last
/// clone to tear down the subprocess.
#[derive(Clone)]
pub struct AcpClient {
    inner: Arc<AcpClientInner>,
}

struct AcpClientInner {
    /// Display name from the spec — used as the `agent` field on every
    /// emitted [`AgentStreamEvent`].
    agent_name: String,
    /// Send prompt requests to the background dispatch task.
    requests: mpsc::Sender<PromptRequest>,
}

impl AcpClient {
    /// Spawn the agent subprocess, run `initialize`, and start the
    /// background dispatch task. Resolves once the handshake completes;
    /// the task keeps running for the lifetime of the returned client.
    pub async fn connect(spec: AcpAgentSpec) -> Result<Self, super::AcpError> {
        let agent_name = spec.name.clone();
        let agent_name_for_task = agent_name.clone();
        let (req_tx, req_rx) = mpsc::channel::<PromptRequest>(32);

        let sdk_agent = AcpAgent::new(spec.to_sdk_config());

        tokio::spawn(async move {
            let task_result: Result<(), agent_client_protocol::Error> =
                Client::default()
                    .connect_with(sdk_agent, async |cx: ConnectionTo<AcpAgentRole>| {
                        run_dispatch_loop(cx, &agent_name_for_task, req_rx).await
                    })
                    .await;

            if let Err(e) = task_result {
                tracing::warn!(error = %e, "acp dispatch loop ended");
            }
        });

        Ok(Self {
            inner: Arc::new(AcpClientInner {
                agent_name,
                requests: req_tx,
            }),
        })
    }

    /// Returns a cheap handle for issuing prompts against this client.
    pub fn session(&self) -> AcpSessionHandle<'_> {
        AcpSessionHandle {
            agent_name: &self.inner.agent_name,
            requests: &self.inner.requests,
        }
    }
}

/// Borrowed view of an [`AcpClient`] for issuing prompts.
///
/// Returned by [`AcpClient::session`]; has the lifetime of the borrow,
/// not of the underlying client.
#[allow(dead_code)] // `agent_name` is reserved for future per-handle labelling.
pub struct AcpSessionHandle<'a> {
    agent_name: &'a str,
    requests: &'a mpsc::Sender<PromptRequest>,
}

impl<'a> AcpSessionHandle<'a> {
    /// Send one prompt to the agent and stream back events.
    ///
    /// Each call opens a fresh ACP session (`session/new`) on the
    /// shared connection — the SDK is per-session stateful, but the
    /// connection and initialize handshake are reused across prompts.
    pub async fn prompt(
        &self,
        text: impl Into<String>,
    ) -> Result<StreamReader<AgentStreamEvent>, super::AcpError> {
        let (tx, rx) = StreamReader::new(64);
        let (write_tx, mut write_rx) = mpsc::channel::<Option<AgentStreamEvent>>(64);

        self.requests
            .send(PromptRequest {
                text: text.into(),
                reply: write_tx,
            })
            .await
            .map_err(|_| super::AcpError::Cancelled)?;

        // Pump events from the SDK side into the StreamReader.
        tokio::spawn(async move {
            while let Some(event) = write_rx.recv().await {
                match event {
                    Some(ev) => {
                        if !tx.send(ev).await {
                            break;
                        }
                    }
                    None => break,
                }
            }
        });

        Ok(rx)
    }
}

/// Inside-`connect_with` body: receive prompt requests over `req_rx`,
/// and for each one spin up an ACP session that streams translated
/// events through `PromptRequest::reply`.
async fn run_dispatch_loop(
    cx: ConnectionTo<AcpAgentRole>,
    agent_name: &str,
    mut req_rx: mpsc::Receiver<PromptRequest>,
) -> Result<(), agent_client_protocol::Error> {
    // `initialize` is the only request we send *before* the first
    // session. From here on we just loop on incoming prompts.
    cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
        .block_task()
        .await?;

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    while let Some(req) = req_rx.recv().await {
        let PromptRequest { text, reply } = req;
        // `drive_one_prompt` always emits a terminal event (Final or
        // error) followed by `None` before returning, so we don't need
        // a separate cleanup path here.
        if let Err(e) = drive_one_prompt(&cx, agent_name, &text, cwd.as_path(), reply).await {
            tracing::warn!(error = %e, "acp prompt failed");
        }
    }

    Ok(())
}

/// Execute one prompt inside one ACP session and forward translated
/// events to `reply`. Emits a terminal `AgentStreamEvent::Final` on
/// success and a sentinel `None` afterwards to close the stream.
async fn drive_one_prompt(
    cx: &ConnectionTo<AcpAgentRole>,
    agent_name: &str,
    text: &str,
    cwd: &std::path::Path,
    reply: mpsc::Sender<Option<AgentStreamEvent>>,
) -> Result<(), agent_client_protocol::Error> {
    let session_result = cx
        .build_session_from(NewSessionRequest::new(cwd.to_path_buf()))
        .block_task()
        .run_until(async |mut session| {
            // Early-return surfaceable errors so the outer `match` can
            // emit a `Final` + close the stream before bubbling up.
            if let Err(e) = session.send_prompt(text) {
                return Err(e);
            }

            // Drain `read_update` until we see a `StopReason` (terminal).
            loop {
                let msg = match session.read_update().await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(error = %e, "acp read_update error");
                        break;
                    }
                };
                let is_terminal = matches!(msg, agent_client_protocol::SessionMessage::StopReason(_));
                if let Some(ev) = translate_session_message(agent_name, msg) {
                    let terminal = ev.is_terminal();
                    if reply.send(Some(ev)).await.is_err() {
                        break;
                    }
                    if terminal {
                        break;
                    }
                } else if is_terminal {
                    break;
                }
            }
            Ok::<_, agent_client_protocol::Error>(())
        })
        .await;

    if let Err(e) = session_result {
        // The session aborted before emitting its terminal event — make
        // sure the consumer sees a `Final` and a `None` so the stream
        // closes cleanly.
        let _ = reply
            .send(Some(AgentStreamEvent::Final {
                agent: agent_name.to_string(),
                message: crate::schema::Message::assistant(format!("[ACP error: {e}]")),
            }))
            .await;
        let _ = reply.send(None).await;
        return Err(e);
    }

    // Happy path — the loop body already pushed the terminal Final
    // (translated from `StopReason`). Just close the producer.
    let _ = reply.send(None).await;
    Ok(())
}

/// Translate one `SessionMessage` into one optional `AgentStreamEvent`.
/// `None` means "no surfaceable event, keep going."
fn translate_session_message(
    agent_name: &str,
    msg: agent_client_protocol::SessionMessage,
) -> Option<AgentStreamEvent> {
    use agent_client_protocol::SessionMessage as Sm;
    match msg {
        Sm::SessionMessage(dispatch) => {
            // `Dispatch` carries either a Notification (a `SessionUpdate`
            // chunk) or a Response. We surface notifications only.
            use agent_client_protocol::Dispatch as D;
            match dispatch {
                D::Notification(notif) => {
                    // `notif` is an untyped `{method, params}` envelope. For
                    // `session/update` the params are a `SessionNotification`
                    // (sessionId + update); some future SDK typings may hand
                    // us a bare `SessionUpdate`, so try both — a failed parse
                    // is logged, never dropped silently.
                    match parse_session_update(&notif.method, notif.params) {
                        Some(update) => {
                            translate_update(agent_name, update).into_iter().next()
                        }
                        None => None,
                    }
                }
                D::Request(_, responder) => {
                    // The agent asked us for something (e.g. fs/read);
                    // we don't service requests yet — respond with a
                    // generic error so the agent doesn't deadlock.
                    let _ = responder.respond_with_error(
                        agent_client_protocol::ErrorCode::InternalError.into(),
                    );
                    None
                }
                D::Response(..) => None,
            }
        }
        Sm::StopReason(stop) => Some(AgentStreamEvent::Final {
            agent: agent_name.to_string(),
            message: crate::schema::Message::assistant(format!(
                "[ACP prompt finished: {stop:?}]"
            )),
        }),
        _ => None, // `#[non_exhaustive]` — ignore future variants.
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse the `params` of an ACP `session/update` notification into a
/// [`SessionUpdate`].
///
/// Tries the [`SessionNotification`] envelope first (the actual wire
/// shape — `{sessionId, update}`), then a bare [`SessionUpdate`] (some
/// future SDK typings may hand us that). Returns `None` and logs at
/// debug level if neither shape parses; never drops silently.
fn parse_session_update(
    method: &str,
    params: serde_json::Value,
) -> Option<agent_client_protocol::schema::v1::SessionUpdate> {
    use agent_client_protocol::schema::v1::{SessionNotification, SessionUpdate};
    let update = serde_json::from_value::<SessionNotification>(params.clone())
        .ok()
        .map(|envelope| envelope.update)
        .or_else(|| serde_json::from_value::<SessionUpdate>(params).ok());
    if update.is_none() {
        tracing::debug!(method = %method, "ignoring unparsable ACP notification");
    }
    update
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{SessionUpdate, SessionNotification};
    use serde_json::json;

    /// Wire shape the agent actually sends for `session/update`: the
    /// params are a `SessionNotification` envelope carrying `sessionId`
    /// + `update`. Before the fix this was parsed as a bare
    /// `SessionUpdate` and silently dropped, so `AgentMessageChunk`
    /// text never reached consumers.
    #[test]
    fn parse_session_update_handles_envelope_shape() {
        let params = json!({
            "sessionId": "sess-1",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "hello envelope" }
            }
        });
        let update = parse_session_update("session/update", params).expect("envelope should parse");
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                agent_client_protocol::schema::v1::ContentBlock::Text(t) => {
                    assert_eq!(t.text, "hello envelope");
                }
                other => panic!("expected text content, got {other:?}"),
            },
            other => panic!("expected AgentMessageChunk, got {other:?}"),
        }
    }

    /// Some future SDK typings may hand us the update without the
    /// envelope — keep that path working too.
    #[test]
    fn parse_session_update_falls_back_to_bare_update() {
        let params = json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "bare shape" }
        });
        let update = parse_session_update("session/update", params).expect("bare should parse");
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                agent_client_protocol::schema::v1::ContentBlock::Text(t) => {
                    assert_eq!(t.text, "bare shape");
                }
                other => panic!("expected text content, got {other:?}"),
            },
            other => panic!("expected AgentMessageChunk, got {other:?}"),
        }
    }

    /// Garbage in → `None` out (and a debug-level trace); never panic,
    /// never silently lose the outer notification.
    #[test]
    fn parse_session_update_returns_none_on_unparsable() {
        let update = parse_session_update("session/update", json!({ "foo": "bar" }));
        assert!(update.is_none(), "garbage must not parse");
    }

    /// Sanity-check the envelope struct itself deserializes the way we
    /// think — guards against SDK renames silently breaking the helper.
    #[test]
    fn session_notification_envelope_round_trips() {
        let raw = json!({
            "sessionId": "s",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "" }
            }
        });
        let env: SessionNotification = serde_json::from_value(raw).unwrap();
        // `SessionId(pub Arc<str>)` — pull the inner `Arc<str>` and deref.
        assert_eq!(&*env.session_id.0, "s");
        assert!(matches!(env.update, SessionUpdate::AgentMessageChunk(_)));
    }
}