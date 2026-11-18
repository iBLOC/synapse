//! Where the client's answer comes from -- and what a failure looks like.
//!
//! ⚠⚠ The single-send path used to read the answer out of
//! `task.history.last()`. That is wrong in two ways, and both of them
//! showed up while talking to Google ADK for real:
//!
//! ❶ **The answer lives in `task.artifacts`.** ADK's own task
//! converter (`google/adk/a2a/converters/to_adk_event.py`) reads the
//! answer from the artifacts and only falls back to `status.message`
//! in the boundary states. Meanwhile synapse's own server puts the
//! **caller's** message into the history and the answer into the
//! artifacts -- so "the last history entry" hands the caller's own
//! words back as the agent's answer.
//!
//! ❷ **A failed task is an error, not an answer.** ADK's converter
//! stamps a failed task with an error code and message instead of
//! handing the text back as content. Reporting "401 UNAUTHENTICATED" as
//! a perfectly well-formed reply is the "failure looks like success"
//! shape: the caller cannot tell a refusal from a reply.

use std::sync::Arc;

use synapse::a2a::card::AgentCard;
use synapse::a2a::server::A2AServerBuilder;
use synapse::adk::agent::{Agent, AgentEvent, AgentInput, AgentOutput, AgentStreamEvent};

/// The agent's answer -- deliberately different from anything the caller
/// says, so "the caller's own words came back" is visible.
const ANSWER: &str = "AGENT_ANSWER";
/// What the caller sends.
const QUESTION: &str = "CALLER_QUESTION";

/// An agent that always answers with `ANSWER`.
struct DistinctAgent;

#[async_trait::async_trait]
impl Agent for DistinctAgent {
    fn name(&self) -> String {
        "distinct".to_string()
    }

    fn description(&self) -> String {
        "answers with a fixed, distinctive text".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant(ANSWER)),
                action: None,
            },
            error: None,
        })
    }

    async fn stream_run(&self, _input: AgentInput) -> synapse::adk::agent::AgentStream {
        let (writer, reader) = synapse::stream::StreamReader::<AgentStreamEvent>::new(4);
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::Final {
                    agent: "distinct".to_string(),
                    message: synapse::schema::Message::assistant(ANSWER),
                })
                .await;
        });
        reader
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: synapse::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        self.run(AgentInput {
            messages: vec![synapse::schema::Message::user("")],
            enable_streaming: false,
        })
        .await
    }
}

/// Start a synapse server on an ephemeral loopback port.
async fn spawn_server(agent: Arc<dyn Agent>) -> std::net::SocketAddr {
    let card = AgentCard::new("distinct", "reply probe", "http://127.0.0.1", "1.0.0");
    A2AServerBuilder::new(agent, card)
        .serve_on("127.0.0.1:0".parse().unwrap())
        .await
        .expect("the server must bind")
}

/// Start a fake peer that answers every call with the given task.
async fn spawn_fake_peer(task: serde_json::Value) -> std::net::SocketAddr {
    let app = axum::Router::new().route(
        "/",
        axum::routing::post(move |body: axum::body::Bytes| {
            let task = task.clone();
            async move {
                let id = serde_json::from_slice::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v.get("id").cloned())
                    .unwrap_or_else(|| serde_json::json!(1));
                axum::Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "task": task }
                }))
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

/// The text of a message.
fn text_of(message: &synapse::schema::Message) -> String {
    match &message.content {
        synapse::schema::Content::Text(text) => text.clone(),
        other => format!("{other:?}"),
    }
}

/// A history entry the **caller** produced.
fn caller_turn() -> serde_json::Value {
    serde_json::json!({
        "messageId": "m-user",
        "role": "ROLE_USER",
        "parts": [{ "text": QUESTION }]
    })
}

/// A history entry the **agent** produced.
fn agent_turn(text: &str) -> serde_json::Value {
    serde_json::json!({
        "messageId": "m-agent",
        "role": "ROLE_AGENT",
        "parts": [{ "text": text }]
    })
}

/// ⚠⚠ The answer is the artifact, not the caller's own words.
///
/// synapse's own server puts the caller's message into the task history
/// and the answer into the artifacts. Reading the last history entry
/// therefore hands the caller's own question back -- which reads like a
/// reply because it is a well-formed message.
#[tokio::test]
async fn the_answer_is_the_artifact_not_the_callers_own_words() {
    let addr = spawn_server(Arc::new(DistinctAgent)).await;
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}/"))
        .await
        .expect("the client must build");

    let reply = client
        .send_message(synapse::schema::Message::user(QUESTION))
        .await
        .expect("the peer must answer");

    assert_eq!(
        text_of(&reply),
        ANSWER,
        "the answer must be the agent's artifact, not the caller's echoed question"
    );
    assert_ne!(
        text_of(&reply),
        QUESTION,
        "the caller's own words must never come back as the agent's answer"
    );
}

/// ⚠⚠ A failed task must be reported as a failure.
///
/// The peer says "401 UNAUTHENTICATED" in the task's status message.
/// Handing that text back as a reply is the "failure looks like
/// success" shape.
#[tokio::test]
async fn a_failed_task_is_a_failure_not_a_reply() {
    let task = serde_json::json!({
        "id": "t-failed",
        "contextId": "c1",
        "status": {
            "state": "TASK_STATE_FAILED",
            "message": {
                "messageId": "m-err",
                "role": "ROLE_AGENT",
                "parts": [{ "text": "401 UNAUTHENTICATED" }]
            }
        },
        "history": [caller_turn()]
    });
    let addr = spawn_fake_peer(task).await;
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}"))
        .await
        .expect("the client must build");

    let result = client
        .send_message(synapse::schema::Message::user(QUESTION))
        .await;

    let error = match result {
        Ok(reply) => panic!(
            "a failed task must not be reported as a reply; the client answered: {}",
            text_of(&reply)
        ),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("401 UNAUTHENTICATED"),
        "the peer's error must survive into the failure; the error was: {error}"
    );
}

/// ⚠⚠ A task with no artifacts still answers -- but with the **agent's**
/// turn, not the caller's.
///
/// Some peers fill the history and leave the artifacts empty. The
/// caller's own turn is in there too, and it must not be mistaken for
/// the answer.
#[tokio::test]
async fn a_task_with_only_history_answers_with_the_agents_turn() {
    let task = serde_json::json!({
        "id": "t-history",
        "contextId": "c1",
        "status": { "state": "TASK_STATE_COMPLETED" },
        "history": [caller_turn(), agent_turn("HISTORY_ANSWER")]
    });
    let addr = spawn_fake_peer(task).await;
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}"))
        .await
        .expect("the client must build");

    let reply = client
        .send_message(synapse::schema::Message::user(QUESTION))
        .await
        .expect("the peer must answer");

    assert_eq!(
        text_of(&reply),
        "HISTORY_ANSWER",
        "the agent's own turn is the answer; the caller's turn is not"
    );
}

/// ⚠⚠ When the peer fills both, the artifact wins.
///
/// This is exactly ADK's shape: the answer is in the artifacts *and* in
/// the history. ADK's own converter reads the artifacts, so that is
/// what the answer must be.
#[tokio::test]
async fn the_artifact_wins_when_the_peer_fills_both() {
    let task = serde_json::json!({
        "id": "t-both",
        "contextId": "c1",
        "status": { "state": "TASK_STATE_COMPLETED" },
        "artifacts": [
            { "artifactId": "a1", "parts": [{ "text": "ARTIFACT_ANSWER" }] }
        ],
        "history": [caller_turn(), agent_turn("HISTORY_ANSWER")]
    });
    let addr = spawn_fake_peer(task).await;
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}"))
        .await
        .expect("the client must build");

    let reply = client
        .send_message(synapse::schema::Message::user(QUESTION))
        .await
        .expect("the peer must answer");

    assert_eq!(
        text_of(&reply),
        "ARTIFACT_ANSWER",
        "the artifacts are the answer source, even when the history repeats it"
    );
}

/// ⚠⚠ A canceled task is not an answer either.
#[tokio::test]
async fn a_canceled_task_is_a_failure_not_a_reply() {
    let task = serde_json::json!({
        "id": "t-canceled",
        "contextId": "c1",
        "status": { "state": "TASK_STATE_CANCELED" },
        "history": [caller_turn()]
    });
    let addr = spawn_fake_peer(task).await;
    let client = synapse::a2a::client::A2AClient::new(&format!("http://{addr}"))
        .await
        .expect("the client must build");

    let result = client
        .send_message(synapse::schema::Message::user(QUESTION))
        .await;

    assert!(
        result.is_err(),
        "a canceled task must not be reported as a reply; the client answered: {:?}",
        result.map(|r| text_of(&r))
    );
}
