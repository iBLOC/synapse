//! What the client's streaming path does when the peer's stream is not
//! the happy path.
//!
//! ⚠⚠ A2A v1.0 has two rules that a streaming client can get wrong, and
//! both of them turn an incomplete exchange into something that looks
//! complete:
//!
//! ❶ **The answer may arrive in the status message.** The wire format
//! lets a peer deliver the answer either as an artifact or as the
//! message of a status update. Google ADK puts it in **both** — so a
//! client that reads both shows the answer twice, and a client that
//! reads only the artifact misses a peer that sends it the other way.
//!
//! ❷ **A stream that ends is not a task that ended.** The task reaches
//! an end only through a terminal state. A stream that simply stops
//! (peer crashed, network dropped) leaves the task open — which is
//! exactly how ADK reads it ("A2A response stream ended before task
//! ... reached a terminal state").

use synapse::adk::agent::AgentStreamEvent;

/// Start a fake peer that streams the given events and then ends.
async fn spawn_streaming_peer(events: Vec<serde_json::Value>) -> std::net::SocketAddr {
    let app = axum::Router::new().route(
        "/",
        axum::routing::post(move |body: axum::body::Bytes| {
            let events = events.clone();
            async move {
                // Echo the request id back: some peers check it.
                let id = serde_json::from_slice::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("id").cloned())
                    .unwrap_or_else(|| serde_json::json!("1"));
                let mut sse = String::new();
                for event in events {
                    let envelope = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": event
                    });
                    sse.push_str("data: ");
                    sse.push_str(&envelope.to_string());
                    sse.push_str("\n\n");
                }
                (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/event-stream",
                    )],
                    sse,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

/// The submitted task event every stream starts with.
fn submitted() -> serde_json::Value {
    serde_json::json!({
        "task": {
            "id": "t1",
            "contextId": "c1",
            "status": { "state": "TASK_STATE_SUBMITTED" }
        }
    })
}

/// A status update with a state and, optionally, a message.
fn status(state: &str, message: Option<&str>) -> serde_json::Value {
    let message = match message {
        Some(text) => serde_json::json!({
            "messageId": "m1",
            "role": "ROLE_AGENT",
            "parts": [{ "text": text }]
        }),
        None => serde_json::Value::Null,
    };
    serde_json::json!({
        "statusUpdate": {
            "taskId": "t1",
            "contextId": "c1",
            "status": { "state": state, "message": message }
        }
    })
}

/// An artifact update carrying text.
///
/// `append` follows the A2A v1.0 rule: `false` creates (or replaces)
/// the artifact, `true` appends to it.
fn artifact(text: &str, append: bool, last_chunk: bool) -> serde_json::Value {
    serde_json::json!({
        "artifactUpdate": {
            "taskId": "t1",
            "contextId": "c1",
            "artifact": {
                "artifactId": "a1",
                "parts": [{ "text": text }]
            },
            "append": append,
            "lastChunk": last_chunk
        }
    })
}

/// The text of a final message.
fn final_text(event: &AgentStreamEvent) -> Option<String> {
    match event {
        AgentStreamEvent::Final { message, .. } => match &message.content {
            synapse::schema::Content::Text(text) => Some(text.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Consume the whole stream and collect the events.
async fn collect(addr: std::net::SocketAddr) -> Vec<AgentStreamEvent> {
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}"))
        .await
        .expect("the client must build");
    let mut stream = client
        .send_message_stream(synapse::schema::Message::user("hi"))
        .await
        .expect("the stream must open");
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }
    events
}

/// The error text of an error event.
fn error_text(event: &AgentStreamEvent) -> Option<String> {
    match event {
        AgentStreamEvent::Error { error, .. } => Some(format!("{error}")),
        _ => None,
    }
}

/// ⚠⚠ The answer may arrive in the status message — and only there.
#[tokio::test]
async fn an_answer_in_the_status_message_still_reaches_the_caller() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        status("TASK_STATE_WORKING", Some("ANSWER_IN_STATUS")),
        status("TASK_STATE_COMPLETED", None),
    ])
    .await;

    let events = collect(addr).await;

    let finals: Vec<String> = events.iter().filter_map(final_text).collect();
    assert_eq!(
        finals,
        vec!["ANSWER_IN_STATUS".to_string()],
        "the answer must reach the caller even when it arrives as a status message; events were {events:?}"
    );
    assert!(
        !events.iter().any(|e| error_text(e).is_some()),
        "a completed task must not be reported as a failure; events were {events:?}"
    );
}

/// ⚠⚠ A stream that ends without a terminal state is a truncated task.
#[tokio::test]
async fn a_stream_that_ends_without_a_terminal_state_is_a_failure() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        artifact("half an answer", false, false),
    ])
    .await;

    let events = collect(addr).await;

    let errors: Vec<String> = events.iter().filter_map(error_text).collect();
    assert_eq!(
        errors.len(),
        1,
        "the truncated stream must be reported once; events were {events:?}"
    );
    assert!(
        errors[0].contains("terminal state"),
        "the failure must say the task never reached a terminal state; the error was: {}",
        errors[0]
    );
    assert!(
        events.iter().all(|e| final_text(e).is_none()),
        "a truncated stream must not be reported as an answer; events were {events:?}"
    );
}

/// ⚠⚠ ADK puts the answer in **both** places — it must show up once.
#[tokio::test]
async fn an_answer_in_both_places_is_not_reported_twice() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        status("TASK_STATE_WORKING", Some("SAME_ANSWER")),
        artifact("SAME_ANSWER", false, true),
        status("TASK_STATE_COMPLETED", None),
    ])
    .await;

    let events = collect(addr).await;

    let finals: Vec<String> = events.iter().filter_map(final_text).collect();
    assert_eq!(
        finals,
        vec!["SAME_ANSWER".to_string()],
        "the answer must be reported exactly once; events were {events:?}"
    );
    assert!(
        !events.iter().any(|e| error_text(e).is_some()),
        "a completed task must not be reported as a failure; events were {events:?}"
    );
}

/// A healthy stream must stay healthy: one answer, no error.
#[tokio::test]
async fn a_healthy_stream_reports_the_answer_once_and_no_failure() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        artifact("hello ", false, false),
        artifact("world", true, true),
        status("TASK_STATE_COMPLETED", None),
    ])
    .await;

    let events = collect(addr).await;

    let finals: Vec<String> = events.iter().filter_map(final_text).collect();
    assert_eq!(
        finals,
        vec!["hello world".to_string()],
        "the answer must reach the caller; events were {events:?}"
    );
    assert!(
        !events.iter().any(|e| error_text(e).is_some()),
        "a healthy stream must not be reported as a failure; events were {events:?}"
    );
}

/// ⚠⚠ A replaced artifact replaces the answer.
///
/// `append = false` means "create or replace" -- so a second update
/// with `append = false` does not add to the first one.
#[tokio::test]
async fn a_replaced_artifact_replaces_the_answer() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        artifact("first", false, false),
        artifact("second", false, true),
        status("TASK_STATE_COMPLETED", None),
    ])
    .await;

    let events = collect(addr).await;

    let finals: Vec<String> = events.iter().filter_map(final_text).collect();
    assert_eq!(
        finals,
        vec!["second".to_string()],
        "the replaced artifact must replace the answer; events were {events:?}"
    );
}

/// A failed task is a failure — and not also a "truncated stream".
#[tokio::test]
async fn a_failed_task_is_reported_once_as_a_failure() {
    let addr = spawn_streaming_peer(vec![
        submitted(),
        status("TASK_STATE_FAILED", Some("401 UNAUTHENTICATED")),
    ])
    .await;

    let events = collect(addr).await;

    let errors: Vec<String> = events.iter().filter_map(error_text).collect();
    assert_eq!(
        errors.len(),
        1,
        "the failure must be reported exactly once; events were {events:?}"
    );
    assert!(
        errors[0].contains("401 UNAUTHENTICATED"),
        "the peer's error must survive into the failure; the error was: {}",
        errors[0]
    );
}
