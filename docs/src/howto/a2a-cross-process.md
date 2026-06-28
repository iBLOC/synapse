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

// Send a task to remote agent
let response = a2a.send_message(
    "https://other-agent.example.com/a2a",
    serde_json::json!({
        "task": "What is the capital of France?",
        "context": {"user_id": "u-123"}
    }),
).await?;

println!("Response: {}", response);
```

### Process B (Server)

Process B needs to expose an HTTP endpoint that accepts A2A
messages. Synapse doesn't ship a built-in A2A server, but any
HTTP server (axum, actix, warp) works:

```rust
use axum::{Json, handler};

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
    // Process task using your Synapse agent
    // ...
    Json(serde_json::json!({
        "result": format!("Processed: {}", task)
    }))
}
```

## Streaming Responses

For long-running tasks, use streaming:

```rust
let stream = a2a.stream_message("https://...", payload).await?;
while let Some(chunk) = stream.next().await {
    println!("Chunk: {:?}", chunk);
}
```

## See Also

- [A2A concepts](../concepts/a2a.md)
- [WASM Sandbox](../concepts/wasm.md) — Run in sandboxed environment
