# A2A Protocol

A2A (Agent-to-Agent) is a protocol for agents to communicate with
each other across processes or machines. Synapse implements A2A
client via the `A2AAdapter`.

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

Synapse plays the **client** role. The server can be any A2A-compliant
agent in any language.

## A2AAdapter

```rust
use std::sync::Arc;
use synapse::protocol::a2a_adapter::A2AAdapter;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> = Arc::new(DefaultHttpClient::new());
let a2a = A2AAdapter::new(http);

// Send A2A messages
a2a.send_message("https://other-agent.example.com/a2a", payload).await?;
```

The adapter reuses `HttpClient`, so it works natively and in WASM
sandboxed environments.

## See Also

- [A2A Cross-Process how-to](../howto/a2a-cross-process.md)
- [WASM Sandbox](./wasm.md) — A2A in WASM contexts
