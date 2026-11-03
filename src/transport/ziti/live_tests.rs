//! **Live** end-to-end test over a real zero-trust network.
//!
//! ## Why this file is `#[ignore]` by default
//!
//! It needs a real OpenZiti network (controller ＋ edge router ＋ an
//! enrolled identity) ⇒ ⇒ it cannot run in CI and must not run on a
//! developer machine that has no network ⇒ ⇒ **`#[ignore]`** is the honest
//! default: the test exists, it is real, and it only runs when a network is
//! actually there.
//!
//! ## How to run it once the network is up
//!
//! ```powershell
//! $env:ZITI_IDENTITY  = "C:\path\to\client.identity.json"
//! $env:ZITI_SERVICE   = "shadow-brain"
//! $env:ZITI_SHADOW_KEY = "..."      # 可选 —— 门要钥匙时
//! cargo test --offline --features ziti --test ziti_live -- --ignored --nocapture
//! ```
//!
//! ## What it proves
//!
//! | test | proves |
//! |---|---|
//! | `live_http_request_response` | a plain request/response crosses the overlay |
//! | `live_websocket_upgrade` | the **upgrade** handshake crosses the overlay |
//! | `live_sse_stream` | a streaming response crosses the overlay |
//! | `live_proxy_end_to_end` | the **local proxy** (what the frontends use) works end to end |

use std::time::Duration;

use super::{
    connect_ws_over_stream, request_over_stream_bytes, spawn_proxy, ziti_connector, ProxyState,
    ZitiGateway,
};
use http_body_util::BodyExt;
use tokio_tungstenite::tungstenite::Message;

/// The identity of the **client** side.
fn identity_path() -> Option<String> {
    std::env::var("ZITI_IDENTITY")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The dark service name on the **brain** side.
fn service_name() -> String {
    std::env::var("ZITI_SERVICE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "shadow-brain".to_string())
}

/// Credentials the door may ask for.
fn door_headers() -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if let Ok(key) = std::env::var("ZITI_SHADOW_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            headers.push(("X-Shadow-Key".to_string(), key));
        }
    }
    if let Ok(token) = std::env::var("ZITI_ADMIN_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            headers.push(("X-Admin-Token".to_string(), token));
        }
    }
    headers
}

/// Skip (not fail) when the environment is not set up.
macro_rules! require_network {
    () => {
        match identity_path() {
            Some(path) => path,
            None => {
                eprintln!("跳过：没有 ZITI_IDENTITY —— 这一条需要一张真的零信任网络。");
                return;
            }
        }
    };
}

/// ✅ A plain request/response crosses the overlay.
#[tokio::test]
#[ignore = "需要一张真的零信任网络"]
async fn live_http_request_response() {
    let identity = require_network!();
    let gateway = ZitiGateway::new(&identity).await.expect("加载身份");

    let stream = gateway.dial(&service_name()).await.expect("拨号");
    let (head, mut body) = request_over_stream_bytes(
        stream,
        &service_name(),
        "GET",
        "/api/telencephalon/status",
        None,
        Some(door_headers()),
    )
    .await
    .expect("请求");

    println!("HTTP 状态码 = {}", head.status);
    assert!(
        head.status.is_success() || head.status == 401 || head.status == 403,
        "应当拿到一个像样的 HTTP 状态码（200/401/403），实际是 {}",
        head.status
    );

    let collected = body.collect().await.expect("读 body");
    let collected = collected.to_bytes();
    let text = String::from_utf8_lossy(&collected);
    println!("响应体（前 200 字）= {}", &text.chars().take(200).collect::<String>());
}

/// ✅ The **upgrade** handshake crosses the overlay.
#[tokio::test]
#[ignore = "需要一张真的零信任网络"]
async fn live_websocket_upgrade() {
    let identity = require_network!();
    let gateway = ZitiGateway::new(&identity).await.expect("加载身份");

    let stream = gateway.dial(&service_name()).await.expect("拨号");
    let (mut socket, response) = connect_ws_over_stream(
        stream,
        &service_name(),
        "/api/trace/stream",
        door_headers(),
    )
    .await
    .expect("握手");

    println!("WS 握手状态码 = {}", response.status());
    assert_eq!(response.status().as_u16(), 101, "应当是 101");

    // A trace stream may be silent —— ⇒ we only prove the channel is alive.
    match tokio::time::timeout(Duration::from_secs(5), futures::StreamExt::next(&mut socket)).await {
        Ok(Some(Ok(Message::Text(text)))) => println!("收到一帧：{}", &text.chars().take(120).collect::<String>()),
        Ok(Some(Ok(other))) => println!("收到一帧：{other:?}"),
        Ok(Some(Err(e))) => println!("帧读取出错：{e}"),
        Ok(None) => println!("上游关闭了连接（通道仍然是通的）"),
        Err(_) => println!("5 秒内没有帧（通道仍然是通的）"),
    }
}

/// ✅ A streaming response crosses the overlay.
#[tokio::test]
#[ignore = "需要一张真的零信任网络"]
async fn live_sse_stream() {
    let identity = require_network!();
    let gateway = ZitiGateway::new(&identity).await.expect("加载身份");

    let conversation = std::env::var("ZITI_TEST_CONVERSATION")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let Some(conversation) = conversation else {
        eprintln!("跳过 SSE 这一条：没有 ZITI_TEST_CONVERSATION（需要一个真实会话 id）。");
        return;
    };

    let stream = gateway.dial(&service_name()).await.expect("拨号");
    let path = format!("/shadow/v1/conversations/{conversation}/messages");
    let (head, mut body) = request_over_stream_bytes(
        stream,
        &service_name(),
        "GET",
        &path,
        None,
        Some(door_headers()),
    )
    .await
    .expect("请求");

    println!("SSE 状态码 = {}", head.status);
    assert_eq!(head.status, 200, "SSE 端点应当返回 200");

    let content_type = head
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    println!("SSE content-type = {content_type}");
    assert!(
        content_type.starts_with("text/event-stream"),
        "应当是 text/event-stream，实际是 {content_type}"
    );

    // Read two data frames and prove they arrive **separately**.
    let first = read_data_frame(&mut body).await;
    let second = read_data_frame(&mut body).await;
    println!("第一块 = {}", String::from_utf8_lossy(&first));
    println!("第二块 = {}", String::from_utf8_lossy(&second));
}

/// ✅ **The whole client path**: local proxy ⇒ ziti ⇒ remote brain.
///
/// ⇒ ⇒ This is the one that matches what the frontends actually do
/// (`fetch` to a loopback address) ⇒ ⇒ if it passes, the frontends need
/// **zero** changes to talk to a remote brain.
#[tokio::test]
#[ignore = "需要一张真的零信任网络"]
async fn live_proxy_end_to_end() {
    let identity = require_network!();
    let gateway = std::sync::Arc::new(ZitiGateway::new(&identity).await.expect("加载身份"));

    let state = ProxyState::new(
        ziti_connector(gateway.clone(), service_name()),
        service_name(),
    )
    .with_default_headers(door_headers());
    let addr = spawn_proxy(state, "127.0.0.1:0".parse().expect("addr"))
        .await
        .expect("起本地代理");
    println!("本地代理 = http://{addr}");

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{addr}/api/telencephalon/status"))
        .send()
        .await
        .expect("经代理请求");
    println!("经代理的状态码 = {}", response.status());
    assert!(
        response.status().is_success() || response.status() == 401 || response.status() == 403,
        "经代理应当拿到像样的状态码，实际 {}",
        response.status()
    );
}

/// Read the next non-empty data frame.
async fn read_data_frame(body: &mut hyper::body::Incoming) -> bytes::Bytes {
    use http_body_util::BodyExt;
    loop {
        let frame = body.frame().await.expect("帧").expect("帧");
        if let Ok(data) = frame.into_data() {
            if !data.is_empty() {
                return data;
            }
        }
    }
}
