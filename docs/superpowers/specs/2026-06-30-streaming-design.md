---
title: Streaming & A2A Server Design
status: Draft
date: 2026-06-30
author: Synapse Contributors
version: 0.1
---

# Streaming & A2A Server Design

> **Scope**: End-to-end streaming (LLM providers, agents, multi-agent, A2A) plus a built-in A2A v1.0 server, all on top of the official `a2a-rs` Rust SDK.

## 1. Goals & Non-Goals

### 1.1 Goals

1. **End-to-end token streaming** — every LLM call surface token deltas to consumers, from the lowest `BaseChatModel::stream` boundary up through `Agent::stream_run`, `MultiAgent` supervisors, and the A2A cross-process boundary.
2. **Unified event bus** — one `AgentStreamEvent` enum drives observability, cancellation, and A2A wire translation; no more ad-hoc `Result<Message, _>` streams.
3. **Built-in A2A v1.0 server** — host any Synapse `Agent` as an A2A peer with axum + JSON-RPC over SSE, fully wire-compatible with Google's ADK, BeeAI, a2a-python, etc.
4. **A2A v1.0 client** — stream + subscribe operations over JSON-RPC and HTTP+JSON/REST bindings, including automatic resubscribe on disconnect.
5. **One unified `StreamReader<T>`** — collapse the existing `model::StreamReader` and `compose::runnable::StreamReader` into a single, cancellation-aware type.
6. **Backward compatible** — `Agent::run()` and existing sync A2A calls keep working unchanged. New APIs are additive.

### 1.2 Non-Goals (this spec)

- gRPC / SLIMRPC bindings (SDK provides, not wired in v1)
- Persistent `TaskStore` (sqlite/postgres) — in-memory only; restart loses tasks
- Authentication implementations (SDK provides `CallInterceptor` hook; we do not implement OAuth2/API-key)
- Agent Card signing (v1.0 §8.4) — defer
- Multi-tenancy (v1.0 §6) — defer
- WebSocket transport for A2A — JSON-RPC over SSE only
- WASM-side A2A client/server — native only in v1

## 2. Background & Context

### 2.1 Current state (master @ `1607f6a`)

- `BaseChatModel::stream` trait defined; only `OpenAIModel` has partial SSE plumbing, `AnthropicClient`/`AzureClient` stubs.
- Two parallel `StreamReader` types: `components::model::StreamReader` (mpsc-based, `Result<Message, ModelError>`) and `compose::runnable::StreamReader<T>` (mpsc-based, `Result<T, Box<dyn Error>>`).
- ReAct loop (`adk/react.rs::run_loop`) and `GraphReActAgent` only call `model.generate()`; no incremental token emission.
- A2A implementation is custom (`src/a2a/{card,protocol,mod}.rs`, `src/protocol/a2a_adapter.rs`) — *not* aligned with the upstream A2A v1.0 spec or `a2a-rs` SDK.

### 2.2 Upstream references

- **A2A v1.0 spec**: <https://a2a-protocol.org/latest/specification/> (Linux Foundation; donated by Google; v1.0.0 released)
- **A2A Rust SDK**: <https://github.com/a2aproject/a2a-rs>
  - `a2a` (core types / JSON-RPC / serde) — package `a2a-lf` v0.3.x
  - `a2a_client` (async client + protocol negotiation) — package `a2a-client-lf` v0.2.x
  - `a2a_server` (axum-based server + SSE) — package `a2a-server-lf` v0.4.x
  - `a2a_pb` (protobuf; **not** used in v1)
  - **MSRV**: Rust 1.85; **edition**: 2024
  - **Default features**: `rustls-tls`

### 2.3 Decisions made during brainstorming (2026-06-30)

| Decision | Choice | Rationale |
|---|---|---|
| Scope | **C — full end-to-end** | Highest ROI; partial scope leaves A2A broken |
| API compatibility | **Backward compatible** (`run()` unchanged, new `stream_run()`) | Zero migration cost; matches LangChain's `__call__`/`stream` split |
| A2A transport | **JSON-RPC over SSE** (aligning with A2A v1.0 default) | Reuses `HttpClient` + `WasiHttpClient`; WASM-friendly when SDK adds it |
| A2A server impl | **Depend on official `a2a-rs` SDK** | Wire compatibility guaranteed; free InMemoryTaskStore, middleware, SSE helpers |
| StreamReader | **Unify** in this spec | Two parallel types are tech debt; cost is ~5 files |
| A2A server v1 scope | **Expanded** (use all SDK-provided features) | Push notification, middleware, list-tasks come free |
| MSRV | **Bump to 1.85** | Required by `a2a-rs` |
| Crate source | **crates.io** (pinned versions) | No Git dependency needed |

## 3. Architecture

### 3.1 Five layers + one cross-cutting bus

```
┌──────────────────────────────────────────────────────────────────────┐
│  Application                                                          │
│    user code: A2AServerBuilder::new(agent, card).bind(addr).serve()   │
│    user code: A2AClient::stream_send(peer, req)                       │
├──────────────────────────────────────────────────────────────────────┤
│  Agent Layer                                                          │
│    Agent::stream_run() / ChatModelAgent::stream_run()                 │
│    GraphReActAgent::stream_run() / MultiAgent supervisors             │
├──────────────────────────────────────────────────────────────────────┤
│  Event Bus  ←──  AgentStreamEvent (one enum, 8 variants, see §4.2)     │
│    TokenDelta · ToolCall · ToolStart · ToolEnd                        │
│    StepTransition · RouteDecision · Handoff · Final · Error           │
├──────────────────────────────────────────────────────────────────────┤
│  Cross-cutting Middleware (opt-in)                                    │
│    TokenCounter · RetryPolicy · BackoffPolicy · ToolFallback          │
├──────────────────────────────────────────────────────────────────────┤
│  Provider Layer                                                       │
│    BaseChatModel::stream() → StreamReader<MessageDelta>               │
│    OpenAIModel · AnthropicModel · AzureOpenAIModel                    │
├──────────────────────────────────────────────────────────────────────┤
│  Transport                                                            │
│    HttpClient::sse_post() (ReqwestHttpClient / WasiHttpClient)        │
└──────────────────────────────────────────────────────────────────────┘
```

### 3.2 Cross-process — A2A layer (separate concern, layered on top)

```
Synapse-side A2A wrapper  →  a2a_server (SDK)  →  axum  →  peer agents
                         ↘  a2a_client (SDK)  →  reqwest / a2a_peer
```

## 4. Core Types

### 4.1 New module layout

| New / changed | Path | Notes |
|---|---|---|
| **NEW** | `src/stream/mod.rs` | Unified `StreamReader<T>`, `StreamWriter<T>`, `CancelHandle` |
| **NEW** | `src/adk/stream/event.rs` | `AgentStreamEvent` enum |
| **NEW** | `src/adk/stream/forward.rs` | `forward_to()` helper |
| **NEW** | `src/components/model/delta.rs` | `MessageDelta`, `ToolCallDelta`, `FinishReason`, `Usage` |
| **DELETE** | `src/compose/runnable/stream.rs` | Replaced by `src/stream/` |
| **DELETE** | `src/components/model/mod.rs::StreamReader`/`StreamWriter` | Re-exported from `src/stream/` |
| **EXTEND** | `src/transport/mod.rs` | Add `sse_post()`, `SseStream`, `SseEvent` |
| **EXTEND** | `src/transport/wasi.rs` | Implement `sse_post` for WASM |
| **EXTEND** | `src/components/model/{openai,anthropic,azure}.rs` | Implement `stream()` over `sse_post` |
| **EXTEND** | `src/adk/agent/mod.rs` | Default `stream_run` impl on `Agent` trait |
| **EXTEND** | `src/adk/react.rs` | Add `ChatModelAgent::stream_run` |
| **EXTEND** | `src/adk/agent/react/graph_react.rs` | Add `GraphReActAgent::stream_run` |
| **EXTEND** | `src/adk/multiagent/{router,orchestrator,handoff,hybrid}.rs` | Each gets `stream_run` |
| **NEW** | `src/a2a/translate.rs` | `AgentStreamEvent` ↔ A2A events |
| **NEW** | `src/a2a/client.rs` | Thin wrapper over `a2a_client::Client` |
| **NEW** | `src/a2a/server.rs` | `SynapseAgentHandler` impl + `A2AServerBuilder` |
| **REWRITE** | `src/a2a/{card,protocol,mod}.rs` | Re-export `a2a` types where possible |
| **REWRITE** | `src/protocol/a2a_adapter.rs` | Thin wrapper over `a2a_client` |

### 4.2 `AgentStreamEvent`

```rust
// src/adk/stream/event.rs

use crate::schema::{Message, ToolCall};
use crate::adk::agent::AgentError;

#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    /// LLM output token delta.
    TokenDelta { agent: String, delta: String },

    /// Fully-formed tool call (after all per-tool deltas accumulated).
    ToolCall { agent: String, call: ToolCall },

    /// Tool execution start (pre-middleware).
    ToolStart { agent: String, name: String, args: String },

    /// Tool execution end (post-middleware).
    ToolEnd {
        agent: String,
        name: String,
        result: String,
        latency_ms: u64,
    },

    /// Internal state transition (ReAct step, Graph node entered/exited).
    StepTransition {
        agent: String,
        from: String,
        to: String,
        step: usize,
    },

    /// Multi-agent routing decision (RouterSupervisor).
    RouteDecision { supervisor: String, target: String, reason: String },

    /// Multi-agent handoff (HandoffSupervisor).
    Handoff { from: String, to: String, context_size: usize },

    /// Successful terminal event.
    Final { agent: String, message: Message },

    /// Failure terminal event.
    Error { agent: String, error: AgentError },
}

pub type AgentStream = StreamReader<AgentStreamEvent>;
```

**Why these 9 variants**:
- `TokenDelta` + `ToolCall` + `ToolStart` + `ToolEnd` — cover the full LLM↔Tool interaction surface
- `StepTransition` — observability for ReAct loops and graph nodes
- `RouteDecision` + `Handoff` — observability for MultiAgent
- `Final` + `Error` — explicit terminal markers; consumers never need to detect "done" via stream closure

**Deliberately excluded** (future extensions):
- `Reasoning` — couples to provider-specific implementations
- `Cost` / `Usage` — belongs in the cross-cutting `TokenCounter` middleware, not the event bus

### 4.3 Unified `StreamReader<T>`

```rust
// src/stream/mod.rs

use futures::Stream;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct StreamReader<T> {
    rx: mpsc::Receiver<T>,
    cancel: Arc<AtomicBool>,
}

pub struct StreamWriter<T> { tx: mpsc::Sender<T> }

#[derive(Clone)]
pub struct CancelHandle { flag: Arc<AtomicBool> }

impl CancelHandle {
    pub fn cancel(&self) { self.flag.store(true, Ordering::SeqCst); }
    pub fn is_cancelled(&self) -> bool { self.flag.load(Ordering::SeqCst) }
}

impl<T> StreamReader<T> {
    pub fn new(cap: usize) -> (StreamWriter<T>, Self) {
        let (tx, rx) = mpsc::channel(cap);
        let cancel = Arc::new(AtomicBool::new(false));
        (StreamWriter { tx }, StreamReader { rx, cancel })
    }

    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle { flag: self.cancel.clone() }
    }

    /// Convert to a boxed `Stream` for use with `futures` combinators.
    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = T> + Send>> {
        Box::pin(self)
    }

    /// Async `next()` (for use without `Stream` trait).
    pub async fn next(&mut self) -> Option<T> { self.rx.recv().await }
}

impl<T: Send + 'static> Stream for StreamReader<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}
```

**Cancellation design**:
- `Arc<AtomicBool>` shared between the reader and any number of `CancelHandle` clones
- Each `await` point in producer/consumer code is responsible for checking `is_cancelled()`
- Producer side drops `StreamWriter` → channel closes → `rx.recv()` returns `None`

### 4.4 `MessageDelta` and friends

```rust
// src/components/model/delta.rs

#[derive(Debug, Clone, Default)]
pub struct MessageDelta {
    pub content_delta: String,
    pub tool_call_deltas: Vec<ToolCallDelta>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments_delta: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
    Error,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}
```

## 5. Layer 1 — Transport (`HttpClient::sse_post`)

### 5.1 Trait extension

```rust
// src/transport/mod.rs (EXTEND)

use tokio::sync::mpsc;

#[async_trait]
pub trait HttpClient: Send + Sync {
    // Existing methods unchanged
    async fn post(&self, url: &str, headers: HeaderMap, body: Vec<u8>) -> Result<HttpResponse, HttpError>;
    async fn get(&self, url: &str, headers: HeaderMap) -> Result<HttpResponse, HttpError>;

    // NEW: SSE streaming POST
    async fn sse_post(
        &self,
        url: &str,
        headers: HeaderMap,
        body: Vec<u8>,
    ) -> Result<SseStream, HttpError>;
}

pub struct SseStream { rx: mpsc::Receiver<Result<SseEvent, SseError>> }

pub struct SseEvent {
    pub event: Option<String>,  // SSE "event:" field
    pub data: String,            // SSE "data:" field
    pub id: Option<String>,      // SSE "id:" field
}

#[derive(Debug, thiserror::Error)]
pub enum SseError {
    #[error("connection lost: {0}")]
    ConnectionLost(String),
    #[error("malformed SSE line: {0}")]
    MalformedLine(String),
    #[error("underlying HTTP error: {0}")]
    Http(HttpError),
}
```

### 5.2 Native implementation (`ReqwestHttpClient`)

Uses `reqwest::Response::bytes_stream()`. The `sse_post` implementation:
1. Sets `Accept: text/event-stream` header
2. Spawns a task that reads `bytes_stream()` chunks, runs them through `sse::parse_chunk()`, and sends `SseEvent`s into a `mpsc::channel`
3. Closes the channel on EOF or HTTP error

### 5.3 WASM implementation (`WasiHttpClient`)

Uses `wasi::http::outgoing_handler::handle()`. The `sse_post` implementation:
1. Same header setup
2. Reads body via `wasi::http::types::IncomingBody::stream()` repeatedly
3. Same `sse::parse_chunk()` parser (pure function, shared with native)

### 5.4 Shared SSE parser

```rust
// src/transport/sse.rs (NEW)

/// Parse a chunk of bytes (possibly containing multiple SSE events) into events.
/// Handles multi-line "data:" fields, BOM, and CRLF / LF line endings.
pub fn parse_chunk(chunk: &[u8], state: &mut SseParseState) -> Vec<SseEvent> { ... }

pub struct SseParseState {
    pub current_event: Option<String>,
    pub current_data: Vec<String>,
    pub current_id: Option<String>,
}
```

`SseParseState` lives in the producer task (not exposed to consumers) so partial chunks across `read()` calls parse correctly.

## 6. Layer 2 — Provider (LLM streaming)

### 6.1 OpenAI / Azure OpenAI

**Wire format** (SSE):

```
data: {"id":"chatcmpl-...","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hi"},"finish_reason":null}]}
data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_xxx","function":{"name":"get_weather","arguments":""}}]}}]}
data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"location\":"}}]}}]}
data: [DONE]
```

**Parser**:
1. For each `data:` line (skip keepalives, blank lines)
2. If `[DONE]` → emit empty `MessageDelta` with `finish_reason=Some(Stop)`, then close
3. Else parse JSON; map `choices[0].delta.content` → `content_delta`; map `choices[0].delta.tool_calls[i]` → `ToolCallDelta` (by `index`)
4. Accumulate `usage` if present in the final chunk (OpenAI supports optional `stream_options: {"include_usage": true}`)

### 6.2 Anthropic

**Wire format** (event-driven):

```
event: message_start
data: {"type":"message_start","message":{"id":"msg_01...","usage":{"input_tokens":25}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"loc"}}

event: content_block_stop
data: {"type":"content_block_start","index":0,...}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}
```

**Parser** is a **state machine**:
- Track `current_block_index` and `current_block_type` (`text` vs `tool_use`)
- On `content_block_delta` of type `text_delta` → `content_delta`
- On `content_block_delta` of type `input_json_delta` → `tool_call_deltas` entry with `arguments_delta` accumulated
- On `content_block_start` of type `tool_use` → set `id` and `name` on the `ToolCallDelta`
- On `message_delta` → capture `stop_reason` and `usage`
- On `message_stop` → emit final `MessageDelta` with `finish_reason`, close stream

### 6.3 `BaseChatModel::stream` contract

```rust
#[async_trait]
pub trait BaseChatModel: Send + Sync {
    async fn generate(&self, input: Vec<Message>, options: GenerateOptions) -> ModelResult<Message>;
    async fn stream(&self, input: Vec<Message>, options: GenerateOptions) -> ModelResult<StreamReader<MessageDelta>>;
}
```

**Producer-task shape** (per provider):

```rust
async fn stream(&self, input, opts) -> ModelResult<StreamReader<MessageDelta>> {
    let (writer, reader) = StreamReader::new(64);
    let cancel = reader.cancel_handle();
    let self_clone = self.clone_arc();
    let input_clone = input.clone();
    let opts_clone = opts.clone();

    tokio::spawn(async move {
        let mut sse = self_clone.http.sse_post(&url, headers, body).await?;
        let mut parser = SseParseState::default();

        while let Some(event_res) = sse.next().await {
            if cancel.is_cancelled() { return Ok(()); }
            let event = event_res?;
            for ev in parse_chunk(&event.data, &mut parser) {
                // provider-specific: openai::parse / anthropic::parse
                let delta = parse_provider_event(&ev)?;
                if writer.send_value(delta).await == false { return Ok(()); }
            }
        }
        Ok(())
    });

    Ok(reader)
}
```

### 6.4 Error handling

| Failure | Handling |
|---|---|
| HTTP 4xx (auth, request) | First `MessageDelta { finish_reason: Some(Error), .. }`, then close |
| HTTP 429 (rate limit) | First `MessageDelta { finish_reason: Some(Error) }`; user attaches `BackoffPolicy` middleware to retry |
| HTTP 5xx | Same as 4xx; user attaches `RetryPolicy` |
| SSE parse error (malformed JSON) | Skip line + log; if >10 consecutive failures, emit `Error` and close |
| Connection reset | `SseError::ConnectionLost` → emit `Error` and close |
| Cancellation | Silent close (no error event) |

## 7. Layer 3 — Agent (`Agent::stream_run` + ReAct)

### 7.1 `Agent` trait extension

```rust
// src/adk/agent/mod.rs (EXTEND)

#[async_trait]
pub trait Agent: Send + Sync {
    // ... existing ...

    /// Stream events for the given input. Default implementation wraps `run()`.
    async fn stream_run(&self, input: AgentInput) -> AgentStream
    where
        Self: Sized,
    {
        AgentStream::from_run(self.run(input).await)
    }
}
```

`AgentStream::from_run` wraps a single `AgentEvent` in a stream that yields one synthetic `Final` (or `Error`) event then closes. This means **every existing `Agent` implementation gets a free `stream_run`** that consumers can use; only built-in agents need to override for true incremental streaming.

### 7.2 `ChatModelAgent::stream_run`

Lives in `src/adk/react.rs`. Reuses the existing `run_loop` logic but splits `model.generate(...)` into `model.stream(...)` and emits events.

**Event sequence for a single ReAct iteration**:
1. `StepTransition { from: "iter_end", to: "model_call", step: N }`
2. `TokenDelta { delta: "..." }` ×N (one per LLM token)
3. If model produced tool calls:
   - `ToolCall { call: <fully assembled> }` (one per call)
   - `ToolStart { name, args }` (one per call)
   - `ToolEnd { name, result, latency_ms }` (one per call, in order)
   - Loop to next iteration
4. If model produced final answer:
   - `Final { message }`

**Pseudocode** (full implementation in `src/adk/react.rs`):

```rust
pub async fn stream_run(&self, input: AgentInput) -> AgentStream {
    let (writer, reader) = StreamReader::new(128);
    let cancel = reader.cancel_handle();
    let agent = self.clone_arc();
    let agent_name = self.name.clone();

    tokio::spawn(async move {
        let mut messages = input.messages;
        if let Some(p) = &agent.system_prompt {
            messages.insert(0, Message::system(p.clone()));
        }
        let mut iter = 0;
        loop {
            if cancel.is_cancelled() { return; }
            if iter >= agent.max_iterations {
                writer.send_value(AgentStreamEvent::Error { agent: agent_name.clone(), error: AgentError::MaxIterationsExceeded }).await;
                return;
            }
            iter += 1;

            writer.send_value(AgentStreamEvent::StepTransition { agent: agent_name.clone(), from: "iter_end".into(), to: "model_call".into(), step: iter }).await;

            // ---- Stream the LLM call ----
            let opts = GenerateOptions { temperature: agent.temperature, tools: Some(agent.get_tool_infos().await), ..Default::default() };
            let mut msg = Message::assistant("");
            let mut s = agent.model.stream(messages.clone(), opts).await.map_err(|e| AgentError::Agent(e.to_string()))?;
            while let Some(delta) = s.next().await {
                if cancel.is_cancelled() { return; }
                if !delta.content_delta.is_empty() {
                    writer.send_value(AgentStreamEvent::TokenDelta { agent: agent_name.clone(), delta: delta.content_delta.clone() }).await;
                    msg.content.push_str(&delta.content_delta);
                }
                for tc_d in &delta.tool_call_deltas {
                    merge_tool_call_delta(&mut msg, tc_d);
                }
                if matches!(delta.finish_reason, Some(FinishReason::Error)) {
                    writer.send_value(AgentStreamEvent::Error { agent: agent_name.clone(), error: AgentError::Agent("stream error".into()) }).await;
                    return;
                }
            }
            messages.push(msg.clone());

            // ---- Tool execution ----
            if let Some(calls) = msg.tool_calls.clone() {
                for call in calls {
                    writer.send_value(AgentStreamEvent::ToolStart { agent: agent_name.clone(), name: call.function.name.clone(), args: call.function.arguments.clone() }).await;
                    let start = std::time::Instant::now();
                    let result = agent.execute_tool(&call).await?;
                    let latency = start.elapsed().as_millis() as u64;
                    writer.send_value(AgentStreamEvent::ToolEnd { agent: agent_name.clone(), name: call.function.name.clone(), result: result.clone(), latency_ms: latency }).await;
                    messages.push(Message { role: RoleType::Tool, content: Content::Text(result), tool_call_id: Some(call.id.clone()), ..Default::default() });
                }
            } else {
                writer.send_value(AgentStreamEvent::Final { agent: agent_name.clone(), message: msg }).await;
                return;
            }
        }
    });

    reader
}
```

### 7.3 `GraphReActAgent::stream_run`

In `src/adk/agent/react/graph_react.rs`:
- New `Graph::execute_stream()` method emits `StepTransition` events on `NodeEntered` / `NodeExited`
- New `LlmNode::execute_stream()` calls `model.stream()` and emits `TokenDelta`
- New `ToolNode::execute_stream()` wraps each tool call in `ToolStart` / `ToolEnd`
- The graph runner itself emits the final `Final` / `Error` after all nodes complete

### 7.4 MultiAgent supervisors

All four supervisors (`RouterSupervisor`, `OrchestratorAgent`, `HandoffSupervisor`, `HybridSupervisor`) get a `stream_run` method following this pattern:

1. Supervisor's own LLM decision (e.g. Router picking a sub-agent) emits the appropriate observability event (`RouteDecision` / `Handoff`)
2. The chosen sub-agent's `stream_run(input)` is invoked
3. Its events are **forwarded transparently** to the supervisor's `AgentStream` via `forward_to()` (see §7.5)
4. When the sub-agent's stream ends, the supervisor may do additional work (e.g. Router may synthesize a final response) and emit `Final` / `Error`

### 7.5 Forwarding helper

```rust
// src/adk/stream/forward.rs

use futures::StreamExt;

/// Forward all events from `inner` to `writer` until inner ends or writer closes.
pub async fn forward_to<T>(mut inner: impl Stream<Item = T> + Unpin, writer: &StreamWriter<T>) {
    while let Some(item) = inner.next().await {
        if writer.send_value(item).await == false { break; }
    }
}
```

### 7.6 Cancellation propagation

Cancel handles are `Arc<AtomicBool>` — cheap to clone across `tokio::spawn` boundaries. The pattern for nested agents:

```rust
let shared_cancel = Arc::new(AtomicBool::new(false));

// Outer agent task
let outer_cancel = shared_cancel.clone();
let outer_stream = outer_agent.stream_run(input).await;
tokio::spawn(async move {
    forward_to(outer_stream, &writer).await;
    if outer_cancel.load(Ordering::SeqCst) {
        // Propagate to inner
    }
});

// Inner sub-agent
let inner_cancel = shared_cancel.clone();
let mut inner_stream = sub_agent.stream_run(sub_input).await;

// Bridge: when shared_cancel is set, also cancel inner
tokio::spawn(async move {
    loop {
        if shared_cancel.load(Ordering::SeqCst) {
            inner_stream.cancel_handle().cancel();
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
});
```

## 8. Layer 4 — Cross-cutting Middleware

Optional `StreamMiddleware` trait for consumers who want to inject cross-cutting behavior:

```rust
// src/adk/middleware/stream.rs (NEW)

pub trait StreamMiddleware: Send + Sync {
    async fn on_event(&self, event: &mut AgentStreamEvent, ctx: &mut StreamContext) -> MiddlewareAction;
}

pub enum MiddlewareAction { Continue, Retry, Skip, Abort }

pub struct StreamContext {
    pub cancel: CancelHandle,
    pub accumulated_usage: Usage,
    // ... future: trace span id, parent call info, etc.
}
```

**v1 includes**:
- `TokenCounter` middleware — populates `accumulated_usage` for cost tracking
- One example showing how to write a `RetryPolicy` middleware

The middleware hook is *opt-in* — agents that don't use it have zero overhead.

## 9. Layer 5 — A2A (client + server, via official `a2a-rs` SDK)

### 9.1 Dependencies (Cargo.toml)

```toml
[dependencies]
a2a        = { package = "a2a-lf",        version = "0.3" }   # core types
a2a_client = { package = "a2a-client-lf", version = "0.2", default-features = false }  # native-tls optional
a2a_server = { package = "a2a-server-lf", version = "0.4", default-features = false }  # native-tls optional

[features]
default = ["rustls-tls"]
rustls-tls   = ["a2a_client/rustls-tls", "a2a_server/rustls-tls"]
native-tls   = ["a2a_client/native-tls", "a2a_server/native-tls"]
```

**MSRV**: bump from 1.75 to 1.85 (required by `a2a-rs`).

**NOT included** in v1:
- `a2a-grpc-lf` (gRPC)
- `a2a-slimrpc-lf` (SLIMRPC)
- `a2a-pb` (protobuf — only used by gRPC/slimrpc)

### 9.2 Translation layer — `src/a2a/translate.rs`

Maps `AgentStreamEvent` ↔ A2A v1.0 events:

| `AgentStreamEvent` | A2A `StreamResponse` |
|---|---|
| `TokenDelta { delta, .. }` | `ArtifactUpdateEvent` with `append: true`, `last_chunk: false` (default artifact name: `"response"`) |
| `ToolStart { name, .. }` | `StatusUpdateEvent` with `state: Working`, message `"→ {name}"` |
| `ToolEnd { name, result, .. }` | `StatusUpdateEvent` with `state: Working`, message `"← {name}: {result}"` |
| `RouteDecision { target, .. }` | `StatusUpdateEvent` with `state: Working`, message `"→ route {target}"` |
| `Handoff { from, to, .. }` | `StatusUpdateEvent` with `state: InputRequired`, message `"handoff {from} → {to}"` |
| `StepTransition { .. }` | (not exposed — internal observability) |
| `Final { message, .. }` | 1. `ArtifactUpdateEvent` with `append: false`, `last_chunk: true`<br>2. `StatusUpdateEvent` with `state: Completed`, `final: true` |
| `Error { error, .. }` | `StatusUpdateEvent` with `state: Failed`, `final: true` |

Reverse direction (A2A → Synapse) is symmetric. The default artifact name is `"response"`; this is overridable via `A2AServerBuilder::default_artifact_name(...)`.

### 9.3 A2A client — `src/a2a/client.rs`

Thin wrapper over `a2a_client::Client`:

```rust
// src/a2a/client.rs

use a2a::types::{SendMessageRequest, SendMessageResponse, StreamResponse, A2AError};
use a2a_client::Client as SdkClient;
use crate::adk::stream::{AgentStream, AgentStreamEvent};
use crate::stream::StreamReader;

pub struct A2AClient {
    sdk: SdkClient,
    peer: String,
}

impl A2AClient {
    pub async fn connect(peer: &str) -> Result<Self, A2AError> {
        let sdk = SdkClient::connect(peer).await?;
        Ok(Self { sdk, peer: peer.into() })
    }

    // Synchronous send (existing capability)
    pub async fn send_message(&self, req: SendMessageRequest) -> Result<SendMessageResponse, A2AError> {
        self.sdk.send_message(req).await
    }

    // NEW: streaming send
    pub async fn stream_send(&self, req: SendMessageRequest) -> Result<AgentStream, A2AError> {
        let (writer, reader) = StreamReader::new(128);
        let cancel = reader.cancel_handle();
        let mut sdk_stream = self.sdk.send_streaming_message(req).await?;

        tokio::spawn(async move {
            while let Some(ev_res) = sdk_stream.next().await {
                if cancel.is_cancelled() { return; }
                match ev_res {
                    Ok(sdk_ev) => {
                        for syn_ev in translate::from_a2a_event(sdk_ev, peer.clone()) {
                            if writer.send_value(syn_ev).await == false { return; }
                        }
                    }
                    Err(e) => {
                        writer.send_value(AgentStreamEvent::Error {
                            agent: "a2a_client".into(),
                            error: e.into(),
                        }).await;
                        return;
                    }
                }
            }
        });

        Ok(reader)
    }

    // NEW: subscribe to existing task (with auto-resubscribe on disconnect)
    pub async fn subscribe_task(&self, task_id: &str) -> Result<AgentStream, A2AError> {
        // Same shape as stream_send, but uses sdk.subscribe_to_task
        // The SDK handles automatic resubscribe on transient disconnects.
    }
}
```

### 9.4 A2A server — `src/a2a/server.rs`

```rust
// src/a2a/server.rs

use a2a::types::*;
use a2a_server::{RequestHandler, ServiceParams, InMemoryTaskStore, TaskStore, AgentCardProducer};
use crate::adk::agent::Agent;
use std::sync::Arc;

/// Adapts a Synapse `Agent` to the SDK's `RequestHandler` interface.
pub struct SynapseAgentHandler {
    agent: Arc<dyn Agent>,
    task_store: Arc<dyn TaskStore>,
    default_artifact_name: String,
}

#[a2a_server::async_trait]
impl RequestHandler for SynapseAgentHandler {
    async fn send_message(&self, _params: &ServiceParams, req: SendMessageRequest) -> Result<SendMessageResponse, A2AError> {
        let task = self.task_store.create(req.message.context_id.clone()).await?;
        let result = self.agent.run(req.message.into()).await
            .map_err(|e| A2AError::internal(e.to_string()))?;
        Ok(translate::agent_event_to_response(result, &task))
    }

    async fn send_streaming_message(
        &self,
        _params: &ServiceParams,
        req: SendMessageRequest,
    ) -> Result<BoxStream<'static, Result<StreamResponse, A2AError>>, A2AError> {
        let task = self.task_store.create(req.message.context_id.clone()).await?;
        let mut agent_stream = self.agent.stream_run(req.message.into()).await;
        let task_id = task.id.clone();
        let context_id = task.context_id.clone();
        let artifact_name = self.default_artifact_name.clone();

        let stream = async_stream::stream! {
            while let Some(ev) = agent_stream.next().await {
                for frame in translate::to_a2a_event(ev, &task_id, &context_id, &artifact_name) {
                    yield Ok(frame);
                }
            }
        };
        Ok(Box::pin(stream))
    }

    async fn get_task(&self, _params: &ServiceParams, req: GetTaskRequest) -> Result<Task, A2AError> {
        self.task_store.get(&req.task_id).await
    }

    async fn list_tasks(&self, _params: &ServiceParams, req: ListTasksRequest) -> Result<ListTasksResponse, A2AError> {
        self.task_store.list(req).await
    }

    async fn cancel_task(&self, _params: &ServiceParams, req: CancelTaskRequest) -> Result<Task, A2AError> {
        self.task_store.cancel(&req.task_id).await
    }

    async fn subscribe_to_task(
        &self,
        _params: &ServiceParams,
        req: SubscribeToTaskRequest,
    ) -> Result<BoxStream<'static, Result<StreamResponse, A2AError>>, A2AError> {
        // SDK's task store provides the subscribe stream
        self.task_store.subscribe(&req.task_id).await
    }

    async fn create_push_config(&self, _params: &ServiceParams, req: TaskPushNotificationConfig) -> Result<TaskPushNotificationConfig, A2AError> {
        self.task_store.create_push_config(req).await
    }
    async fn get_push_config(&self, _params: &ServiceParams, _req: GetTaskPushNotificationConfigRequest) -> Result<TaskPushNotificationConfig, A2AError> { todo!() }
    async fn list_push_configs(&self, _params: &ServiceParams, _req: ListTaskPushNotificationConfigsRequest) -> Result<ListTaskPushNotificationConfigsResponse, A2AError> { todo!() }
    async fn delete_push_config(&self, _params: &ServiceParams, _req: DeleteTaskPushNotificationConfigRequest) -> Result<(), A2AError> { todo!() }
    async fn get_extended_agent_card(&self, _params: &ServiceParams, _req: GetExtendedAgentCardRequest) -> Result<AgentCard, A2AError> { todo!() }
}

/// User-facing builder for the A2A server.
pub struct A2AServerBuilder {
    agent: Arc<dyn Agent>,
    card: AgentCard,
    bind_addr: SocketAddr,
    default_artifact_name: String,
}

impl A2AServerBuilder {
    pub fn new(agent: Arc<dyn Agent>, card: AgentCard) -> Self { ... }
    pub fn bind(mut self, addr: SocketAddr) -> Self { ... }
    pub fn default_artifact_name(mut self, name: impl Into<String>) -> Self { ... }

    pub async fn serve(self) -> Result<(), A2AError> {
        let handler = Arc::new(SynapseAgentHandler {
            agent: self.agent,
            task_store: Arc::new(InMemoryTaskStore::new()),
            default_artifact_name: self.default_artifact_name,
        });
        a2a_server::Server::new(handler)
            .with_agent_card(self.card)
            .bind(self.bind_addr)
            .serve()
            .await
    }
}
```

### 9.5 v1 A2A scope (expanded per user decision)

| Capability | Included | Notes |
|---|---|---|
| `message/send` (JSON-RPC) | ✅ | |
| `message/stream` (JSON-RPC + SSE) | ✅ | |
| `tasks/get` | ✅ | |
| `tasks/list` | ✅ | SDK provides |
| `tasks/cancel` | ✅ | |
| `tasks/subscribe` (with resubscribe) | ✅ | |
| `tasks/pushConfig` CRUD | ✅ | Default-disabled at server level (opt-in via `A2AServerBuilder`) |
| `agent-card.json` discovery | ✅ | `GET /.well-known/agent-card.json` |
| Extended agent card | ❌ (stub) | Defer to v2 |
| gRPC binding | ❌ | Not in deps |
| SLIMRPC binding | ❌ | Not in deps |
| Persistent `TaskStore` (sqlite/postgres) | ❌ | InMemoryTaskStore only |
| Auth implementations (OAuth2/API key) | ❌ | `CallInterceptor` hook exposed; users implement |
| Agent Card signing | ❌ | Defer |
| Multi-tenancy | ❌ | Defer |

### 9.6 WASM limitations

Both `a2a_client` and `a2a_server` (upstream SDK) use `reqwest` and `axum` — both native-only. v1 explicitly documents:

> A2A client and server are **native-only**. WASM-side A2A support will be added once the upstream `a2a-rs` SDK ships a `wasi::http`-based client. Until then, A2A peer communication is unavailable from WASM-compiled Synapse binaries.

### 9.7 Refactor of existing A2A code

| File | Action | New state |
|---|---|---|
| `src/a2a/card.rs` | **Rewrite** to wrap `a2a::types::AgentCard` | ~50 LoC |
| `src/a2a/protocol.rs` | **Rewrite** to re-export `a2a` types | ~30 LoC |
| `src/a2a/mod.rs` | **Rewrite** to expose `client`, `server`, `translate` submodules | ~30 LoC |
| `src/protocol/a2a_adapter.rs` | **Rewrite** as a thin compatibility shim around `A2AClient` (preserves `protocol::AgentAdapter` impl) | ~80 LoC |
| `src/protocol/mod.rs` | **Keep** `AgentAdapter` trait + `ProtocolAdapterRegistry` | Unchanged |

Public API users (callers of `A2AAdapter::send_task`, `A2AAdapter::get_task`, `A2AAdapter::cancel_task`) **see no breaking changes**.

## 10. Testing Strategy

### 10.1 Unit tests (in-module)

```rust
// In src/stream/mod.rs
#[tokio::test] async fn stream_reader_send_recv() { ... }
#[tokio::test] async fn stream_reader_cancel_propagates() { ... }
#[tokio::test] async fn stream_writer_clone_works() { ... }

// In src/transport/sse.rs
#[test] fn parse_chunk_complete_event() { ... }
#[test] fn parse_chunk_multiline_data() { ... }
#[test] fn parse_chunk_split_across_chunks() { ... }
#[test] fn parse_chunk_keepalive_lines() { ... }

// In src/components/model/openai.rs
#[tokio::test] async fn parse_openai_text_stream() { ... }
#[tokio::test] async fn parse_openai_tool_call_stream() { ... }
#[tokio::test] async fn parse_openai_done_marker() { ... }

// In src/components/model/anthropic.rs
#[tokio::test] async fn parse_anthropic_text_stream() { ... }
#[tokio::test] async fn parse_anthropic_text_and_tool_parallel() { ... }
#[tokio::test] async fn parse_anthropic_stop_reasons() { ... }
```

### 10.2 Integration tests (`tests/`)

| Test file | Coverage |
|---|---|
| `tests/llm_stream_test.rs` | OpenAI/Anthropic/Azure SSE end-to-end against `mockito` SSE server |
| `tests/agent_stream_test.rs` | `ChatModelAgent::stream_run` happy path, tool calls, cancellation |
| `tests/graph_react_stream_test.rs` | `GraphReActAgent::stream_run` with multi-step + tool |
| `tests/multiagent_stream_test.rs` | Router, Orchestrator, Handoff, Hybrid streaming |
| `tests/a2a_stream_test.rs` | A2A client `stream_send` and `subscribe_task` against a2a-server-lf `helloworld` |
| `tests/a2a_server_test.rs` | `A2AServerBuilder::serve()` with a test agent, verified by a2a-cli |
| `tests/a2a_e2e_test.rs` | Two Synapse agents (one client, one server) full round-trip |
| `tests/middleware_stream_test.rs` | `TokenCounter`, `RetryPolicy` example middlewares |

### 10.3 Mocking strategy

- `mockito` for HTTP/SSE mocks (already in dev-dependencies)
- `a2a_server::test_util::CapturingHandler` (from upstream SDK) for A2A server-side mocking
- A2A client test setup: spin up `a2a_server::Server` on a free port (loopback only) with a stub handler, point `A2AClient` at it

### 10.4 Verification

Before any PR is merged:
- `cargo build --workspace` must pass
- `cargo test --workspace` must pass (all new and existing tests)
- `cargo clippy --workspace -- -D warnings` must pass
- `cargo fmt --all -- --check` must pass
- A2A v1.0 wire compatibility: at least one test that sends a `message/stream` request with curl and parses the SSE response

## 11. Migration Plan

### 11.1 Module-level changes (in dependency order)

1. Add `a2a`, `a2a_client`, `a2a_server` to `Cargo.toml`; bump MSRV to 1.85
2. Create `src/stream/` with unified `StreamReader<T>` + `CancelHandle`
3. Update `src/components/model/mod.rs` to re-export from `src/stream/`
4. Delete `src/compose/runnable/stream.rs`; update `src/compose/runnable/mod.rs` to re-export
5. Extend `src/transport/mod.rs` with `sse_post` + `SseStream` + `SseEvent`
6. Create `src/transport/sse.rs` with shared parser
7. Implement `sse_post` on `ReqwestHttpClient` and `WasiHttpClient`
8. Create `src/components/model/delta.rs` with `MessageDelta` etc.
9. Implement `stream()` on `OpenAIModel`, `AnthropicModel`, `AzureOpenAIModel`
10. Create `src/adk/stream/event.rs` with `AgentStreamEvent`
11. Add default `stream_run` to `Agent` trait
12. Add `ChatModelAgent::stream_run`
13. Add `GraphReActAgent::stream_run` + `Graph::execute_stream` + `LlmNode::execute_stream`
14. Add `stream_run` to each of the 4 MultiAgent supervisors
15. Rewrite `src/a2a/{card,protocol,mod}.rs` to wrap SDK types
16. Create `src/a2a/translate.rs`
17. Create `src/a2a/client.rs`
18. Create `src/a2a/server.rs`
19. Rewrite `src/protocol/a2a_adapter.rs` as a thin compat shim
20. Tests for each layer

### 11.2 Backward compatibility

- All existing public APIs (`Agent::run`, `BaseChatModel::generate`, `A2AAdapter::send_task`, etc.) remain unchanged
- `Agent::stream_run` has a default implementation, so any custom `Agent` implementation gets a free (non-incremental) stream entry point
- `a2a-rs` dependency is additive; no existing code paths change behavior

### 11.3 Test gate per phase

Each numbered migration step above must pass:
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

before the next step is started.

## 12. Open Questions

1. **A2A Artifact naming** — should `response` be configurable per-supervisor, or globally? *Proposed*: per-server via `A2AServerBuilder::default_artifact_name`. *Action*: defer to implementation; if needed, expose per-call.
2. **Token usage in `StreamResponse`** — A2A v1.0 doesn't define a usage field, but the SDK's `StreamResponse` may pass through a metadata field. *Action*: stuff `Usage` into A2A `metadata` JSON object, document as a Synapse convention.
3. **Cost tracking surface** — should `TokenCounter` middleware emit a separate `Usage` event? *Proposed*: no — usage is queryable from the `StreamContext` after the stream ends; not a stream event.
4. **`tool_choice: None` for streaming** — some providers (Anthropic) require an explicit tool-use mode toggle for streaming. *Action*: pass through `GenerateOptions::tool_choice`; document provider-specific behavior.
5. **Backpressure policy** — current `mpsc::channel(64)` for `BaseChatModel::stream`; what's the right cap? *Proposed*: 64 (covers most cases without head-of-line blocking). Revisit if real workloads show it.
6. **A2A v1.0 wire compatibility test** — should we add a test that uses Google's official ADK Python SDK as a peer? *Proposed*: yes, but defer to post-v1 (requires Python in CI).

## 13. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `a2a-rs` SDK breaks API between minor versions | Med | High | Pin to `version = "0.4"` (semver-compatible within minor until 1.0); weekly CI check; vendored copy as fallback |
| `a2a-rs` 1.0 release changes everything | High | High | Track upstream closely; absorb breaking changes in a dedicated PR; semver-major bump for Synapse |
| SSE parser mismatch for Anthropic's event-driven model | Low | Med | Extensive unit tests + fixtures captured from real API; integration test against live mock |
| MSRV bump to 1.85 excludes users on older toolchains | Low | Low | Document in CHANGELOG; the project is pre-1.0 so MSRV bumps are expected |
| `tokio::mpsc` head-of-line blocking under load | Low | Med | Configurable channel cap; document tuning; consider `tokio_stream::broadcast` in v2 if needed |
| Cancellation races (consumer drops mid-event) | Med | Low | `Arc<AtomicBool>` is well-defined; producer checks at every await point |
| A2A peer doesn't support streaming (older server) | Med | Med | Capability negotiation; fallback to `message/send` + warning log |

## 14. References

- A2A v1.0 specification: <https://a2a-protocol.org/latest/specification/>
- A2A Rust SDK: <https://github.com/a2aproject/a2a-rs>
- A2A streaming tutorial (Python, but wire-compatible): <https://a2a-protocol.org/latest/tutorials/python/7-streaming-and-multiturn/>
- ReAct paper (for ReAct loop semantics): <https://arxiv.org/abs/2210.03629>
- LangChain streaming patterns: <https://python.langchain.com/docs/concepts/streaming/>
- Synapse roadmap: `docs/roadmap.md` (0.2.x "Streaming response support for all chat models")
