---
title: Cross-Process A2A
---

# 跨进程 A2A

跨进程边界向远程 A2A agent 发送任务。

## 目标

连接运行在不同进程或不同机器上的两个 Synapse（或任何符合 A2A 规范的）agent。

## 架构

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Process A   │  ─────────────────→  │  Process B   │
│  (Synapse    │  (A2A over HTTP)    │  (Synapse    │
│   client)    │  ←─────────────────  │   server)    │
└──────────────┘                       └──────────────┘
```

## 搭建

### 进程 A（客户端）

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

### 进程 B（服务端）

进程 B 需要暴露一个 HTTP endpoint。可使用任意框架（axum、actix、warp）：

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
