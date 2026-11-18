//! End-to-end A2A v1.0 extension negotiation over a real socket.

use std::sync::Arc;

use synapse::a2a::card::AgentCard;
use synapse::a2a::extension::{HEADER_A2A_EXTENSIONS, HEADER_X_A2A_EXTENSIONS};
use synapse::a2a::server::A2AServerBuilder;
use synapse::adk::agent::{Agent, AgentEvent, AgentInput, AgentOutput, AgentStreamEvent};

/// A minimal agent that emits one text delta and then a final message.
struct EchoAgent;

#[async_trait::async_trait]
impl Agent for EchoAgent {
    fn name(&self) -> String {
        "echo".to_string()
    }

    fn description(&self) -> String {
        "test agent".to_string()
    }

    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, synapse::adk::agent::AgentError> {
        Ok(AgentEvent {
            agent_name: "echo".to_string(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant("hi")),
                action: None,
            },
            error: None,
        })
    }

    /// ⚠⚠ 回显**入参**：这样「请求的字到了没有」可以直接从响应里读出来。
    /// 回一句固定文本的话，响应里出现什么都证明不了请求到了。
    async fn stream_run(
        &self,
        input: AgentInput,
    ) -> synapse::adk::agent::AgentStream {
        let text = match input.messages.first().map(|m| &m.content) {
            Some(synapse::schema::Content::Text(t)) => t.clone(),
            _ => String::new(),
        };
        let (writer, reader) = synapse::stream::StreamReader::<AgentStreamEvent>::new(4);
        tokio::spawn(async move {
            let _ = writer
                .send(AgentStreamEvent::Final {
                    agent: "echo".to_string(),
                    message: synapse::schema::Message::assistant(text.as_str()),
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
        Ok(AgentEvent {
            agent_name: "echo".to_string(),
            output: AgentOutput {
                message: Some(synapse::schema::Message::assistant("hi")),
                action: None,
            },
            error: None,
        })
    }
}

/// Start a server on an ephemeral loopback port and return its address.
async fn spawn_server(extensions: Vec<String>) -> std::net::SocketAddr {
    let mut card = AgentCard::new("echo", "test agent", "http://127.0.0.1", "1.0.0");
    card.capabilities.extensions = extensions;
    let builder = A2AServerBuilder::new(Arc::new(EchoAgent), card);
    // serve_on binds and reports the *actual* address, so the test can
    // talk to a port nobody else is using. It serves in the background,
    // so this returns as soon as the address is known.
    builder
        .serve_on("127.0.0.1:0".parse().unwrap())
        .await
        .unwrap()
}

/// ⚠⚠ 本条判据钉的是「头真的穿过网络到了服务端」。
/// 单元测试只能证明函数对。
///
/// 路径选流式（`SendStreamingMessage`）而不是单发：
/// ⇒ ⇒ 单发路径返回的是从 task_store 重新加载的 Task 对象，
/// 而 Task 的 metadata 是 store 决定的 ⇒ ⇒ 流式路径才是事件自己带 metadata。
/// synapse 本来就是流式设计的 ⇒ ⇒ 这也是真正的主路径。
#[tokio::test]
async fn the_extension_header_reaches_the_server_over_the_wire() {
    let ext = "https://example.com/ext/v1".to_string();
    let addr = spawn_server(vec![ext.clone()]).await;
    let url = format!("http://{addr}/");

    let resp = post_message(&url, HEADER_X_A2A_EXTENSIONS, &ext).await;
    assert!(resp.status().is_success());
    let body = resp.text().await.expect("body must be readable");

    // ⚠⚠ 消息本身必须真的到了 —— 线格式写错时（oneof 成员在
    // protobuf-JSON 里是**摊平**的，写成 {"content": {...}} 会被当成
    // 未知字段丢掉），请求照样成功，只是字一个都没到。
    assert!(
        body.contains("probe-text"),
        "the message text must reach the server; body was: {body}"
    );

    // 响应里必须看得到「这个扩展被采纳了」。
    let activated = find_metadata_array(&body, "activatedExtensions");
    assert_eq!(
        activated,
        Some(vec![ext.clone()]),
        "the activated extension list must be on the wire; body was: {body}"
    );
}

/// 请求了不支持的扩展 ⇒ ⇒ 必须报出来，不得消失。
#[tokio::test]
async fn an_unsupported_extension_is_reported_over_the_wire() {
    let addr = spawn_server(Vec::new()).await;
    let url = format!("http://{addr}/");

    let resp = post_message(
        &url,
        HEADER_A2A_EXTENSIONS,
        "https://example.com/ext/v1",
    )
    .await;
    let body = resp.text().await.expect("body must be readable");

    let activated = find_metadata_array(&body, "activatedExtensions");
    assert_eq!(
        activated,
        None,
        "nothing may be activated when the agent supports no extension; body was: {body}"
    );
    let unsupported = find_metadata_array(&body, "unsupportedExtensions");
    assert_eq!(
        unsupported,
        Some(vec!["https://example.com/ext/v1".to_string()]),
        "the unsupported extension must be reported; body was: {body}"
    );
}

/// 卡片必须真的声明它支持什么 ⇒ ⇒ 客户端才能据此作出选择。
#[tokio::test]
async fn the_served_card_publishes_the_extensions_and_v1() {
    let ext = "https://example.com/ext/v1".to_string();
    let addr = spawn_server(vec![ext.clone()]).await;
    let url = format!("http://{addr}/.well-known/agent-card.json");

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .expect("the card must be served");
    assert!(resp.status().is_success());
    let value: serde_json::Value = resp.json().await.expect("the card must be JSON");

    // 协议版本必须是 1.0
    assert_eq!(value["supportedInterfaces"][0]["protocolVersion"], "1.0");
    // 扩展必须在卡片里
    let extensions = value["capabilities"]["extensions"]
        .as_array()
        .expect("the card must publish its extensions");
    assert_eq!(extensions.len(), 1);
    assert_eq!(extensions[0]["uri"], ext);
}

/// ⚠⚠ client 侧的承诺：它必须把两个头都发出去。
/// 只发一个就有一类对端看不到请求。
#[tokio::test]
async fn the_synapse_client_sends_both_extension_header_spellings() {
    let seen = Arc::new(std::sync::Mutex::new(std::collections::HashMap::<String, Vec<String>>::new()));
    let seen2 = seen.clone();
    let app = axum::Router::new().route(
        "/",
        axum::routing::post(move |headers: axum::http::HeaderMap, _body: axum::body::Bytes| {
            let seen = seen2.clone();
            async move {
                let mut map = seen.lock().unwrap();
                for (name, value) in headers.iter() {
                    let v = value.to_str().unwrap_or("").to_string();
                    map.entry(name.as_str().to_lowercase())
                        .or_default()
                        .push(v);
                }
                axum::Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {}
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let ext = "https://example.com/ext/v1".to_string();
    let client = synapse::a2a::client::A2AClient::with_extensions(
        &format!("http://{addr}"),
        vec![ext.clone()],
    )
    .await
    .expect("the client must build");

    // 发一个请求（失败不重要，重要的是头到了不不到）。
    let _ = client.send_message(synapse::schema::Message::user("hi")).await;

    let map = seen.lock().unwrap();
    let spec = map
        .get(HEADER_A2A_EXTENSIONS)
        .expect("the spec header A2A-Extensions must be sent");
    assert_eq!(spec, &vec![ext.clone()]);
    let legacy = map
        .get(HEADER_X_A2A_EXTENSIONS)
        .expect("the ADK header X-A2A-Extensions must be sent too");
    assert_eq!(legacy, &vec![ext]);
}

/// POST a streaming message with the given extension header.
async fn post_message(url: &str, header: &str, value: &str) -> reqwest::Response {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "SendStreamingMessage",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "ROLE_USER",
                "parts": [{ "text": "probe-text" }]
            }
        }
    });
    reqwest::Client::new()
        .post(url)
        // ⚠⚠ A2A v1.0 §3.6.1/§3.6.2：缺这个头时按 "0.3" 解释，
        // 而 0.3 不被 v1.0 支持 ⇒ ⇒ 裸请求直接被拒。
        .header("A2A-Version", "1.0")
        .header(header, value)
        .json(&body)
        .send()
        .await
        .expect("the request must reach the server")
}

/// Find a metadata array by key anywhere in an SSE body.
///
/// The body is a sequence of `data: {json}` lines; each line is scanned
/// for the key. Returns the first match.
fn find_metadata_array(body: &str, key: &str) -> Option<Vec<String>> {
    for line in body.lines() {
        let Some(json) = line.strip_prefix("data:") else {
            continue;
        };
        let json = json.trim();
        if json.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
            continue;
        };
        // The event metadata lives under the event variant:
        // result.artifactUpdate.metadata / result.statusUpdate.metadata.
        let event_meta = &value["result"]["artifactUpdate"]["metadata"];
        let event_meta = if event_meta.is_null() {
            &value["result"]["statusUpdate"]["metadata"]
        } else {
            event_meta
        };
        let Some(arr) = event_meta[key].as_array() else {
            continue;
        };
        return Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
        );
    }
    None
}
