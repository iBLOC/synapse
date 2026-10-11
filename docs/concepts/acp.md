---
title: ACP Client
---

# ACP Client

The [`synapse::acp`](../../src/acp/mod.rs) module drives external coding agents that speak the [Agent Client Protocol][acp] (opencode, claude-code-acp, codex-acp, …) over stdio and surfaces their output as a normal Synapse [`AgentStreamEvent`](../../src/adk/stream/mod.rs) stream.

> **Scope** — currently **client-only**: Synapse speaks *to* an external ACP agent and consumes its responses. Implementing the ACP `Agent` role inside Synapse is out of scope for this module.

[acp]: https://agentclientprotocol.com/

## Wire model

```
┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
│ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
│ AcpClient      │                        │ (external subprocess)   │
└────────────────┘                        └────────────────────────┘
```

The SDK spawns the subprocess for us (via [`AcpAgent`][sdk-agent]); we never touch `tokio::process::Command` directly. Each [`AcpClient::session()`](../../src/acp/client.rs) returns a borrowed handle — the underlying `agent-client-protocol` session is opened fresh inside `prompt()` (one session per prompt on the same shared stdio transport).

> **Note** — `AcpSession` is a public re-export alias of `AcpSessionHandle` (see [`src/acp/session.rs`](../../src/acp/session.rs)). Both names refer to the same type; this document uses `AcpSession` for brevity.

[sdk-agent]: https://docs.rs/agent-client-protocol/latest/agent_client_protocol/struct.AcpAgent.html

## Why the official SDK

Four correctness traps we explicitly avoided by depending on `agent-client-protocol` instead of hand-rolling a JSON-RPC client:

1. **Protocol version negotiation** — the SDK downgrades/refreshes based on `InitializeResponse.protocol_version`. A hand-rolled client would freeze on `V1` and silently break against v2 agents.
2. **Request/response correlation** — `SentRequest::block_task().await` plus internal request-id bookkeeping eliminates the "which response belongs to which request" race. DIY clients almost always get this wrong eventually.
3. **`$/cancelRequest` semantics** — UI cancel round-trips correctly even when a proxy sits in front. DIY clients typically forget the JSON-RPC cancel notification and leak subprocess work after stop.
4. **stdio framing** — `Stdio` handles `Content-Length` headers vs line-delimited frames across agents. `AsyncBufReadExt::lines()` does not.

Bonus forward-compatibility the SDK buys us for free:

- `Conductor` / `Proxy` roles in `role::acp` are ready for multi-agent fan-out; we don't need to redesign the connection layer when shadow-desktop starts driving opencode + claude-code-acp side-by-side.
- `with_mcp_server` lets Synapse inject its own MCP servers into a session — agents call our tools without us opening a second port.
- `SessionUpdate` is `#[non_exhaustive]`, which forces `translate_update` to go through `serde_json` round-trip. New schema variants fall through to the `_ => Vec::new()` arm instead of breaking compilation.

## Build & feature gate

Gated by `feature = "acp"` (default **off**) and **non-wasm only** — the upstream SDK does not expose `AcpAgent` or `Stdio` on `target_arch = "wasm32"`.

```bash
cargo build  -p synapse --features acp
cargo test   -p synapse --features acp --lib acp
```

## Quick start

```rust,no_run
# #[cfg(feature = "acp")]
# async fn run() -> Result<(), synapse::acp::AcpError> {
use synapse::acp::{AcpAgentSpec, AcpClient};

// Talk to `opencode --acp` (or any other ACP agent).
let client = AcpClient::connect(AcpAgentSpec::opencode()).await?;

// Send one prompt; consume the stream of `AgentStreamEvent`s.
let mut events = client.session().prompt("Summarise this file").await?;
while let Some(ev) = events.next().await {
    // ToolStart / ToolEnd / TokenDelta / FinalAnswer — same shape as a native agent.
}
# Ok(()) }
```

## Public surface

| Item | Type | Purpose |
|------|------|------|
| `AcpClient` | `struct` (cheap-clone via `Arc`) | One connected client to one external ACP agent |
| `AcpClient::connect(spec)` | `async fn` | Spawn subprocess + `initialize` handshake + start dispatch task |
| `AcpClient::session()` | `fn -> AcpSessionHandle<'_>` | Borrowed handle for issuing prompts |
| `AcpSessionHandle::prompt(text)` *(a.k.a. `AcpSession::prompt`)* | `async fn -> Result<StreamReader<AgentStreamEvent>, AcpError>` | Send one prompt, stream events |
| `AcpAgentSpec` | `struct` (with builder) | Pure-data description of the agent subprocess |
| `AcpAgentSpec::opencode()` | preset | `opencode --acp` |
| `AcpAgentSpec::claude_code()` | preset | `npx -y @agentclientprotocol/claude-agent-acp@latest` |
| `AcpAgentSpec::codex()` | preset | `npx -y @agentclientprotocol/codex-acp@latest` |
| `AcpAgentSpec::new(name, cmd)` | builder | Custom command |
| `AcpAgentSpec::arg(...)` / `.args(...)` / `.env(k, v)` | builder | Extend the spec |
| `AcpError::Sdk` | enum variant | Underlying SDK failure |
| `AcpError::Cancelled` | enum variant | Caller cancelled via `CancelHandle` |

## Custom agent

```rust
use std::path::PathBuf;
use synapse::acp::AcpAgentSpec;

let spec = AcpAgentSpec::new("my-coder", "/usr/local/bin/my-coder-acp")
    .arg("--workspace")
    .arg("/srv/repo")
    .env("OPENAI_API_KEY", "sk-...")
    .env("MY_CODER_MODE", "strict");

let client = synapse::acp::AcpClient::connect(spec).await?;
```

The builder accumulates args / env on top of the inherited environment — you only need to specify the variables the agent actually needs.

## `SessionUpdate` → `AgentStreamEvent` mapping

The translator in [`src/acp/translate.rs`](../../src/acp/translate.rs) turns the ACP SDK's wire-level types into Synapse's streaming primitives:

| ACP source | `AgentStreamEvent` |
|------|------|
| `SessionUpdate::Pending` / `InProgress` (tool) | `ToolStart { name, input, agent }` |
| `SessionUpdate::Completed` / `Failed` (tool) | `ToolEnd { name, output, status, agent }` |
| `AgentMessageChunk` | `TokenDelta { text, agent }` |
| `StopReason::EndTurn` | `Final { agent, message }` |

Nine unit tests cover `AcpAgentSpec` builder behaviour, the `SessionUpdate → AgentStreamEvent` translator, and the `SessionNotification` envelope parser (envelope / bare-update / unparsable / round-trip).

## Migration posture

Callers depend only on `AcpClient` / `AcpSession` (`AcpSessionHandle`) / `AcpAgentSpec` / `AcpError`. Wire-level types (`SessionUpdate`, `ToolCallStatus`, …) are hidden behind `translate.rs`, so a future SDK major bump (v3) that renames or restructures those types is a localized change to the translator — the public API stays put.

## SDK caveat

ACP ships an official Rust SDK — unlike OpenZiti — so there is **no vendor-strategy caveat** here. The crate (`agent-client-protocol`, Apache-2.0) is the official binding maintained at [agentclientprotocol/rust-sdk][sdk]. Pin to `2.2` for now; bump on minor releases only after reading the changelog, since the public API is still pre-1.0.

[sdk]: https://github.com/agentclientprotocol/rust-sdk

## Known gaps

- **Outbound `Dispatch::Request`** from the agent (e.g. fs/read) is answered with a generic `InternalError`. When the time comes to let agents call back into Synapse, plug `Builder::on_receive_request` into the `connect_with` chain.
- **Capability advertisement** — we always send `ProtocolVersion::V1` regardless of what the agent advertised. If we start depending on v2 features (`V2SessionBuilder`, MCP-over-ACP), switch `Client::default()` to `Client::default().v2()` so negotiation fails loud when the agent is too old.

## See also

- [`src/acp/mod.rs`](../../src/acp/mod.rs) — module-level rustdoc (wire model + quick start)
- [`src/acp/client.rs`](../../src/acp/client.rs) — `AcpClient` / `AcpSessionHandle` impl
- [`src/acp/spec.rs`](../../src/acp/spec.rs) — `AcpAgentSpec` builder + presets
- [`src/acp/translate.rs`](../../src/acp/translate.rs) — `SessionUpdate` → `AgentStreamEvent`
- [`src/adk/stream/mod.rs`](../../src/adk/stream/mod.rs) — `AgentStreamEvent` reference
- [Agent Client Protocol spec][acp]
- [agent-client-protocol Rust SDK][sdk]
