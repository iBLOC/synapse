//! A2A task lifecycle over a real socket.
//!
//! ⚠⚠ 本文件钉的是**任务真的走到了终态** ——
//! 这不是风格问题。Google ADK 的 `RemoteA2aAgent` 把
//! `submitted` / `working` 读成「流在中途被截断」并抛错
//! （`google/adk/a2a/agent/_remote_a2a_agent.py`：
//! `_UNFINISHED_TASK_STATES` + "A2A response stream ended before task
//! ... reached a terminal state"），而不是把答复交回去。
//!
//! ⇒ ⇒ 所以「答复到了但任务没结束」在 ADK 眼里等于**失败**。

use std::sync::Arc;

use synapse::a2a::card::AgentCard;
use synapse::a2a::server::A2AServerBuilder;
use synapse::adk::agent::{Agent, AgentEvent, AgentInput, AgentOutput, AgentStreamEvent};

/// An agent that ends with a final message.
struct FinalAgent;

#[async_trait::async_trait]
impl Agent for FinalAgent {
    fn name(&self) -> String {
        "final_agent".to_string()
    }

    fn description(&self) -> String {
        "ends with a final message".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant("hi")),
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
                    agent: "final_agent".to_string(),
                    message: synapse::schema::Message::assistant("hi"),
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

/// An agent that streams a token delta before the final message — the shape
/// a real chat agent has.
struct StreamingAgent;

#[async_trait::async_trait]
impl Agent for StreamingAgent {
    fn name(&self) -> String {
        "streaming_agent".to_string()
    }

    fn description(&self) -> String {
        "streams a delta, then a final message".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant("hi")),
                action: None,
            },
            error: None,
        })
    }

    async fn stream_run(&self, _input: AgentInput) -> synapse::adk::agent::AgentStream {
        let (writer, reader) = synapse::stream::StreamReader::<AgentStreamEvent>::new(4);
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::TokenDelta {
                    agent: "streaming_agent".to_string(),
                    delta: "hi".to_string(),
                })
                .await;
            let _ = writer
                .send(AgentStreamEvent::Final {
                    agent: "streaming_agent".to_string(),
                    message: synapse::schema::Message::assistant("hi"),
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

/// An agent whose stream ends without ever producing a final message.
struct SilentAgent;

#[async_trait::async_trait]
impl Agent for SilentAgent {
    fn name(&self) -> String {
        "silent_agent".to_string()
    }

    fn description(&self) -> String {
        "never produces a final message".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: self.name(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant("hi")),
                action: None,
            },
            error: None,
        })
    }

    async fn stream_run(&self, _input: AgentInput) -> synapse::adk::agent::AgentStream {
        let (writer, reader) = synapse::stream::StreamReader::<AgentStreamEvent>::new(4);
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::TokenDelta {
                    agent: "silent_agent".to_string(),
                    delta: "partial".to_string(),
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

/// Start a server on an ephemeral loopback port and return its address.
async fn spawn_server(agent: Arc<dyn Agent>) -> std::net::SocketAddr {
    let card = AgentCard::new(
        "lifecycle",
        "lifecycle probe",
        "http://127.0.0.1",
        "1.0.0",
    );
    let builder = A2AServerBuilder::new(agent, card);
    builder
        .serve_on("127.0.0.1:0".parse().unwrap())
        .await
        .expect("the server must bind")
}

/// The request body, in the A2A v1.0 protobuf-JSON shape.
///
/// ⚠⚠ `parts` 里是 `{"text": "..."}` —— oneof 成员在 protobuf-JSON 里
/// 是**摊平**的。写成 `{"content": {"text": "..."}}` 会被当成未知字段
/// 丢掉 ⇒ ⇒ 消息看起来"送到了"，其实一个字都没到。
fn message_params(text: &str) -> serde_json::Value {
    serde_json::json!({
        "message": {
            "messageId": "m1",
            "role": "ROLE_USER",
            "parts": [{ "text": text }]
        }
    })
}

/// POST a JSON-RPC call and parse the JSON-RPC envelope.
async fn rpc(url: &str, method: &str, params: serde_json::Value) -> serde_json::Value {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    let resp = reqwest::Client::new()
        .post(url)
        // ⚠⚠ A2A v1.0 `3.6.1/`3.6.2：缺这个头时按 "0.3" 解释，
        // 而 0.3 不被 v1.0 支持 ⇒ ⇒ 裸请求直接被拒。
        .header("A2A-Version", "1.0")
        .json(&body)
        .send()
        .await
        .expect("the request must reach the server");
    let text = resp.text().await.expect("the body must be readable");
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("the response must be a JSON-RPC envelope: {text} ({e})"))
}

/// POST a JSON-RPC call and return the raw (SSE) body.
async fn rpc_stream(url: &str, method: &str, params: serde_json::Value) -> String {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    let resp = reqwest::Client::new()
        .post(url)
        .header("A2A-Version", "1.0")
        .json(&body)
        .send()
        .await
        .expect("the request must reach the server");
    resp.text().await.expect("the body must be readable")
}

/// Every task state that appears in an SSE body, in order.
fn states_in(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let Some(json) = line.strip_prefix("data:") else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json.trim()) else {
            continue;
        };
        let result = &value["result"];
        for key in ["statusUpdate", "task"] {
            let state = &result[key]["status"]["state"];
            if let Some(state) = state.as_str() {
                out.push(state.to_string());
            }
        }
    }
    out
}

/// ⚠⚠ **终态必须真的到终态** ——
///
/// 客户端（ADK 就在其中）按 A2A 规范读任务状态：`submitted` / `working`
/// 意味着「流在中途断了」⇒ ⇒ 它抛错而不是交回答复。
#[tokio::test]
async fn the_task_reaches_a_terminal_state_over_the_wire() {
    let addr = spawn_server(Arc::new(FinalAgent)).await;
    let url = format!("http://{addr}/");

    let value = rpc(&url, "SendMessage", message_params("hi")).await;
    let task = &value["result"]["task"];
    assert_eq!(
        task["status"]["state"], "TASK_STATE_COMPLETED",
        "the task must reach a terminal state; envelope was: {value}"
    );
    assert_eq!(
        task["artifacts"][0]["parts"][0]["text"], "hi",
        "the answer must be on the wire; envelope was: {value}"
    );
}

/// 服务端存下来的任务也必须是终态 —— 客户端会轮询它。
#[tokio::test]
async fn the_stored_task_is_terminal() {
    let addr = spawn_server(Arc::new(FinalAgent)).await;
    let url = format!("http://{addr}/");

    let sent = rpc(&url, "SendMessage", message_params("hi")).await;
    let id = sent["result"]["task"]["id"]
        .as_str()
        .expect("the task must have an id")
        .to_string();

    let fetched = rpc(&url, "GetTask", serde_json::json!({ "id": id })).await;
    assert_eq!(
        fetched["result"]["status"]["state"], "TASK_STATE_COMPLETED",
        "the persisted task must be terminal; envelope was: {fetched}"
    );
}

/// 流式路径也必须以终态收尾 —— 订阅者靠它知道「结束了」。
#[tokio::test]
async fn the_stream_ends_with_a_terminal_status() {
    let addr = spawn_server(Arc::new(FinalAgent)).await;
    let url = format!("http://{addr}/");

    let body = rpc_stream(&url, "SendStreamingMessage", message_params("hi")).await;
    let states = states_in(&body);
    assert!(
        states.iter().any(|s| s == "TASK_STATE_COMPLETED"),
        "the stream must end in a terminal status; states were {states:?}; body was: {body}"
    );
}

/// ⚠⚠ 卡片必须**说实话**：这个 server 确实提供 `SendStreamingMessage`。
///
/// 卡片写 `streaming: false` ⇒ ⇒ 客户端退回单发路径，而 A2A v1.0 的
/// 单发路径返回的是从 store 重新加载的 Task ⇒ ⇒ 事件上的 metadata
/// （扩展协商的结果）整个丢掉。
#[tokio::test]
async fn the_card_advertises_streaming() {
    let addr = spawn_server(Arc::new(FinalAgent)).await;
    let url = format!("http://{addr}/.well-known/agent-card.json");

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect("the card must be served");
    let value: serde_json::Value = resp.json().await.expect("the card must be JSON");
    assert_eq!(
        value["capabilities"]["streaming"], true,
        "the card must advertise the streaming this server actually serves; card was: {value}"
    );
}


/// ⚠⚠ 真的流式代理（先吐 token、再收尾）也必须走完整个生命周期 ——
///
/// 这是真实聊天代理的形状，不是边缘情况。
#[tokio::test]
async fn a_streaming_agent_reaches_a_terminal_state() {
    let addr = spawn_server(Arc::new(StreamingAgent)).await;
    let url = format!("http://{addr}/");

    let value = rpc(&url, "SendMessage", message_params("hi")).await;
    let task = &value["result"]["task"];
    assert_eq!(
        task["status"]["state"], "TASK_STATE_COMPLETED",
        "a streaming agent must also reach a terminal state; envelope was: {value}"
    );
    assert_eq!(
        task["artifacts"][0]["parts"][0]["text"], "hi",
        "the streamed answer must be on the wire; envelope was: {value}"
    );
}

/// ⚠⚠ 代理没给出最终答复 ⇒ ⇒ 任务必须**失败**，不得报「完成」。
///
/// 报完成就是**失败长得像成功** ⇒ ⇒ 客户端会拿到一个空的成功。
#[tokio::test]
async fn an_agent_without_a_final_message_fails_the_task() {
    let addr = spawn_server(Arc::new(SilentAgent)).await;
    let url = format!("http://{addr}/");

    let value = rpc(&url, "SendMessage", message_params("hi")).await;
    let task = &value["result"]["task"];
    assert_eq!(
        task["status"]["state"], "TASK_STATE_FAILED",
        "a stream that never produced a final message must fail the task; envelope was: {value}"
    );
}
