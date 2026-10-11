---
title: A2A Protocol
---

# A2A Protocol

A2A (Agent-to-Agent) is a protocol for agents to communicate with
each other across processes or machines. Synapse implements both
A2A client and server.

## Why A2A?

In multi-agent systems, agents often need to:

- Send tasks to specialized remote agents
- Coordinate workflows across process boundaries
- Discover and invoke agent capabilities

A2A provides a standard protocol (JSON over HTTP) for these scenarios.

## Architecture

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Synapse     │  ─────────────────→  │  A2A Server  │
│  A2A Client  │  (JSON-RPC-like)    │  (Other      │
│              │  ←─────────────────  │   process)   │
└──────────────┘                       └──────────────┘
```

## A2A Client

```rust
use synapse::a2a::A2AClient;

let client = A2AClient::new("http://localhost:8080").await?;
let response = client.send_message(message).await?;
```

## A2A Server

Host a Synapse agent as an A2A server:

```rust
use synapse::a2a::server::A2AServerBuilder;
use synapse::a2a::AgentCard;

let card = AgentCard::new("my-agent", "A helpful assistant", "http://0.0.0.0:8080", "1.0");
A2AServerBuilder::new(agent, card)
    .bind("0.0.0.0:8080".parse()?)
    .serve()
    .await?;
```

### Custom Task Store

By default, the server uses `InMemoryTaskStore`. For persistence:

```rust
use synapse::a2a::store::FileTaskStore;

let store = FileTaskStore::new("/var/lib/my-agent/tasks").await?;
A2AServerBuilder::new(agent, card)
    .serve_with_store(store)
    .await?;
```

`FileTaskStore` persists tasks as JSON files and survives restarts.

## TaskStore Trait

Implement custom persistence by implementing the `TaskStore` trait:

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

## See Also

- [A2A Cross-Process how-to](/howto/a2a-cross-process)
- [WASM Sandbox](/concepts/wasm) — A2A in WASM contexts
