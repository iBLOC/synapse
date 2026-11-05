//! 起一个「真的大脑面」—— 给那 4 条只拨号的 #[ignore] 测试当对面。
//!
//! 用法：
//!   $env:ZITI_HOST_IDENTITY = "...host.identity.json"
//!   $env:ZITI_SERVICE       = "shadow-brain"
//!   cargo run --offline --features ziti --example ziti_v2_brain
//!
//! ⚠️ 它跑的是**生产**那条 serve 入口（synapse::transport::ziti::serve），
//!    不是另写一份。

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use synapse::transport::ziti;

async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "ok": true, "surface": "shadow-brain" }))
}

async fn ws(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            if socket.send(msg).await.is_err() {
                break;
            }
        }
    })
}

async fn sse() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::unfold(0u32, |i| async move {
        if i >= 3 {
            return None;
        }
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
        Some((Ok(Event::default().data(format!("tick-{i}"))), i + 1))
    });
    Sse::new(stream)
}

#[tokio::main]
async fn main() {
    let identity = std::env::var("ZITI_HOST_IDENTITY").expect("需要 ZITI_HOST_IDENTITY");
    let service = std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "shadow-brain".into());

    let app = axum::Router::new()
        .route("/api/telencephalon/status", get(status))
        .route("/api/trace/stream", get(ws))
        .route("/shadow/v1/conversations/{id}/messages", get(sse));

    println!("[brain] 起大脑面：service={service}");
    ziti::serve(app, &identity, &service).await.expect("大脑面起不来");
}
