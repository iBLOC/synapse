---
title: A2A Protocol
---

# A2A 协议

A2A（Agent-to-Agent）是 agent 之间跨进程、跨机器相互通信的协议。Synapse 同时实现了 A2A client 与 server。

## 为什么需要 A2A？

在多 agent 系统中，agent 经常需要：

  - 发送任务给专门化的远程 agent
  - 跨进程边界协调 workflow
  - 发现并调用 agent 的能力

A2A 为这些场景提供了标准协议（JSON over HTTP）。

## 架构

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Synapse     │  ─────────────────→  │  A2A Server  │
│  A2A Client  │  (JSON-RPC-like)    │  (Other      │
│              │  ←─────────────────  │   process)   │
└──────────────┘                       └──────────────┘
```

## A2A 客户端

```rust
use synapse::a2a::A2AClient;

let client = A2AClient::new("http://localhost:8080").await?;
let response = client.send_message(message).await?;
```

## A2A 服务端

把 Synapse agent 托管为 A2A server：

```rust
use synapse::a2a::server::A2AServerBuilder;
use synapse::a2a::AgentCard;

let card = AgentCard::new("my-agent", "A helpful assistant", "http://0.0.0.0:8080", "1.0");
A2AServerBuilder::new(agent, card)
    .bind("0.0.0.0:8080".parse()?)
    .serve()
    .await?;
```

### 自定义 Task Store

默认使用 `InMemoryTaskStore`。如需持久化：

```rust
use synapse::a2a::store::FileTaskStore;

let store = FileTaskStore::new("/var/lib/my-agent/tasks").await?;
A2AServerBuilder::new(agent, card)
    .serve_with_store(store)
    .await?;
```

`FileTaskStore` 把 task 持久化为 JSON 文件，重启后仍然有效。

## 任务存储 trait

实现 `TaskStore` trait 即可自定义持久化：

```rust
use synapse::a2a::store::TaskStore;

struct MyCustomStore { ... }

#[async_trait]
impl TaskStore for MyCustomStore {
    async fn create(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn update(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> { ... }
    async fn list(&self, req: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> { ... }
}
```

## 相关阅读

  - [WASM Sandbox](/zh/concepts/wasm) —— WASM 上下文中的 A2A
