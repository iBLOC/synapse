---
title: Cross-Process A2A
---

# Cross-Process A2A

Send tasks to remote A2A agents across process boundaries.

## Goal

Connect two Synapse (or any A2A-compliant) agents running in
different processes or machines.

## Architecture

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Process A   │  ─────────────────→  │  Process B   │
│  (Synapse    │  (A2A over HTTP)    │  (Synapse    │
│   client)    │  ←─────────────────  │   server)    │
└──────────────┘                       └──────────────┘
```

## Setup

### Process A (Client)

```rust
use std::sync::Arc;
use synapse::protocol::a2a_adapter::A2AAdapter;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> =
    Arc::new(DefaultHttpClient::new());
let a2a = A2AAdapter::new(http);

let response = a2a.send_message(
    "https://other-agent.example.com/a2a",
    serde_json::json!({
        "task": "What is the capital of France?",
    }),
).await?;
```

### Process B (Server)

Process B needs to expose an HTTP endpoint. Use any framework (axum, actix, warp):

```rust
use axum::Json;

#[tokio::main]
async fn main() {
    let app = axum::Router::new()
        .route("/a2a", axum::routing::post(a2a_handler));

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn a2a_handler(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let task = payload["task"].as_str().unwrap_or("");
    Json(serde_json::json!({
        "result": format!("Processed: {}", task)
    }))
}
```

## See Also

- [A2A concepts](/concepts/a2a)
- [WASM Sandbox](/concepts/wasm) — Run in sandboxed environment
