---
title: ACP Client
---

# ACP Client · ACP 客户端

The [`synapse::acp`](../../src/acp/mod.rs) module drives external coding agents that speak the [Agent Client Protocol][acp] (opencode, claude-code-acp, codex-acp, …) over stdio and surfaces their output as a normal Synapse [`AgentStreamEvent`](../../src/adk/stream/mod.rs) stream.

`synapse::acp` 模块驱动通过 [Agent Client Protocol][acp] 与 Synapse 对话的外部编码 agent（opencode、claude-code-acp、codex-acp …），走 stdio JSON-RPC，并把输出映射为常规的 Synapse [`AgentStreamEvent`](../../src/adk/stream/mod.rs) 流。

> **Scope / 范围** — currently **client-only**: Synapse speaks *to* an external ACP agent and consumes its responses. Implementing the ACP `Agent` role inside Synapse is out of scope for this module.
> *目前仅做 **client-only**：Synapse 主动 *连接* 外部 ACP agent 并消费其响应。在 Synapse 内部实现 ACP `Agent` 角色不在本模块范围内。*

[acp]: https://agentclientprotocol.com/

## Wire model · 协议模型

```
┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
│ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
│ AcpClient      │                        │ (external subprocess)   │
└────────────────┘                        └────────────────────────┘
```

The SDK spawns the subprocess for us (via [`AcpAgent`][sdk-agent]); we never touch `tokio::process::Command` directly. Each [`AcpClient::session()`](../../src/acp/client.rs) returns a borrowed handle — the underlying `agent-client-protocol` session is opened fresh inside `prompt()` (one session per prompt on the same shared stdio transport).

> **Note / 备注** — `AcpSession` is a public re-export alias of `AcpSessionHandle` (see [`src/acp/session.rs`](../../src/acp/session.rs)). Both names refer to the same type; this document uses `AcpSession` for brevity.
> *`AcpSession` 是 `AcpSessionHandle` 的公开 re-export 别名（见 [`src/acp/session.rs`](../../src/acp/session.rs)）。两个名字指向同一类型；本文为简洁起见用 `AcpSession`。*

*SDK 由 [`AcpAgent`][sdk-agent] 负责 spawn 子进程；我们不直接碰 `tokio::process::Command`。每次 [`AcpClient::session()`](../../src/acp/client.rs) 返回的都是借用句柄；底层 `agent-client-protocol` session 在 `prompt()` 内部按调用逐次新建（同一 stdio transport 上每个 prompt 开一个 session）。*

[sdk-agent]: https://docs.rs/agent-client-protocol/latest/agent_client_protocol/struct.AcpAgent.html

## Why the official SDK · 为什么用官方 SDK

Four correctness traps we explicitly avoided by depending on `agent-client-protocol` instead of hand-rolling a JSON-RPC client:
*四个正确性陷阱由官方 SDK 处理，我们不再手写 JSON-RPC 客户端：*

1. **Protocol version negotiation** — the SDK downgrades/refreshes based on `InitializeResponse.protocol_version`. A hand-rolled client would freeze on `V1` and silently break against v2 agents.
   *协议版本协商 —— SDK 根据 `InitializeResponse.protocol_version` 自动升降级。手写客户端会卡在 `V1`，遇到 v2 agent 就静默坏掉。*
2. **Request/response correlation** — `SentRequest::block_task().await` plus internal request-id bookkeeping eliminates the "which response belongs to which request" race. DIY clients almost always get this wrong eventually.
   *请求/响应关联 —— `SentRequest::block_task().await` 加上 SDK 内部的请求 ID 簿记消除了"哪个响应属于哪个请求"的竞态。手写客户端几乎都会在这上面栽。*
3. **`$/cancelRequest` semantics** — UI cancel round-trips correctly even when a proxy sits in front. DIY clients typically forget the JSON-RPC cancel notification and leak subprocess work after stop.
   *`$/cancelRequest` 语义 —— UI 取消能在前端有代理的情况下正确回路。手写客户端经常忘记发 JSON-RPC cancel 通知，导致子进程停不掉。*
4. **stdio framing** — `Stdio` handles `Content-Length` headers vs line-delimited frames across agents. `AsyncBufReadExt::lines()` does not.
   *stdio framing —— `Stdio` 帮我们处理 `Content-Length` header vs line-delimited frame 两种编码，`AsyncBufReadExt::lines()` 只能处理后者。*

Bonus forward-compatibility the SDK buys us for free:
*SDK 还免费赠送这些前向兼容能力：*

- `Conductor` / `Proxy` roles in `role::acp` are ready for multi-agent fan-out; we don't need to redesign the connection layer when shadow-desktop starts driving opencode + claude-code-acp side-by-side.
  *`Conductor` / `Proxy` 角色已为多 agent fan-out 备好；shadow-desktop 同时驱动 opencode + claude-code-acp 时不用改连接层。*
- `with_mcp_server` lets Synapse inject its own MCP servers into a session — agents call our tools without us opening a second port.
  *`with_mcp_server` 允许 Synapse 把自己的 MCP server 注入到 session —— agent 调用我们的工具不需要多开一个端口。*
- `SessionUpdate` is `#[non_exhaustive]`, which forces `translate_update` to go through `serde_json` round-trip. New schema variants fall through to the `_ => Vec::new()` arm instead of breaking compilation.
  *`SessionUpdate` 是 `#[non_exhaustive]`，迫使 `translate_update` 走 `serde_json` 往返；新 schema variant 会落进 `_ => Vec::new()` 分支而不是直接编不过。*

## Build & feature gate · 构建与 feature 开关

Gated by `feature = "acp"` (default **off**) and **non-wasm only** — the upstream SDK does not expose `AcpAgent` or `Stdio` on `target_arch = "wasm32"`.
*默认 **off**，仅在 non-wasm 目标编译 —— 上游 SDK 不在 `target_arch = "wasm32"` 上导出 `AcpAgent` / `Stdio`。*

```bash
cargo build  -p synapse --features acp
cargo test   -p synapse --features acp --lib acp
```

## Quick start · 快速开始

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

## Public surface · 公开 API

| Item / 项目 | Type / 类型 | Purpose / 用途 |
|------|------|------|
| `AcpClient` | `struct` (cheap-clone via `Arc`) | One connected client to one external ACP agent / 持有一条外部 ACP agent 连接的客户端 |
| `AcpClient::connect(spec)` | `async fn` | Spawn subprocess + `initialize` handshake + start dispatch task / 起子进程、跑握手、起 dispatch 任务 |
| `AcpClient::session()` | `fn -> AcpSessionHandle<'_>` | Borrowed handle for issuing prompts / 借用句柄用于发起 prompt |
| `AcpSessionHandle::prompt(text)` *(a.k.a. `AcpSession::prompt`)* | `async fn -> Result<StreamReader<AgentStreamEvent>, AcpError>` | Send one prompt, stream events / 发一个 prompt，流式返回事件 |
| `AcpAgentSpec` | `struct` (with builder) | Pure-data description of the agent subprocess / 描述子进程的纯数据 |
| `AcpAgentSpec::opencode()` | preset | `opencode --acp` / 调用 opencode |
| `AcpAgentSpec::claude_code()` | preset | `npx -y @agentclientprotocol/claude-agent-acp@latest` / 调用 claude-code |
| `AcpAgentSpec::codex()` | preset | `npx -y @agentclientprotocol/codex-acp@latest` / 调用 codex |
| `AcpAgentSpec::new(name, cmd)` | builder | Custom command / 自定义命令 |
| `AcpAgentSpec::arg(...)` / `.args(...)` / `.env(k, v)` | builder | Extend the spec / 扩展 spec |
| `AcpError::Sdk` | enum variant | Underlying SDK failure / SDK 底层错误 |
| `AcpError::Cancelled` | enum variant | Caller cancelled via `CancelHandle` / 调用方通过 `CancelHandle` 取消 |

## Custom agent · 自定义 agent

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
*builder 把 args / env 叠加在继承的环境变量上 —— 你只需声明 agent 真正需要的变量。*

## `SessionUpdate` → `AgentStreamEvent` mapping · 事件映射

The translator in [`src/acp/translate.rs`](../../src/acp/translate.rs) turns the ACP SDK's wire-level types into Synapse's streaming primitives:

| ACP source / ACP 来源 | `AgentStreamEvent` |
|------|------|
| `SessionUpdate::Pending` / `InProgress` (tool) | `ToolStart { name, input, agent }` |
| `SessionUpdate::Completed` / `Failed` (tool) | `ToolEnd { name, output, status, agent }` |
| `AgentMessageChunk` | `TokenDelta { text, agent }` |
| `StopReason::EndTurn` | `Final { agent, message }` |

Nine unit tests cover `AcpAgentSpec` builder behaviour, the `SessionUpdate → AgentStreamEvent` translator, and the `SessionNotification` envelope parser (envelope / bare-update / unparsable / round-trip).
*九个单测覆盖 `AcpAgentSpec` builder、`SessionUpdate → AgentStreamEvent` 翻译器、以及 `SessionNotification` envelope 解析（envelope / 裸 update / 不可解析 / 往返）。*

## Migration posture · 升级路径

Callers depend only on `AcpClient` / `AcpSession` (`AcpSessionHandle`) / `AcpAgentSpec` / `AcpError`. Wire-level types (`SessionUpdate`, `ToolCallStatus`, …) are hidden behind `translate.rs`, so a future SDK major bump (v3) that renames or restructures those types is a localized change to the translator — the public API stays put.
*调用方只依赖 `AcpClient` / `AcpSession`（`AcpSessionHandle`）/`AcpAgentSpec` / `AcpError`。线协议类型（`SessionUpdate`、`ToolCallStatus` …）被 `translate.rs` 隔离，SDK 未来大版本（v3）改名/重构只是翻译器局部改动，公开 API 不动。*

## SDK caveat · SDK 注意事项

ACP ships an official Rust SDK — unlike OpenZiti — so there is **no vendor-strategy caveat** here. The crate (`agent-client-protocol`, Apache-2.0) is the official binding maintained at [agentclientprotocol/rust-sdk][sdk]. Pin to `2.2` for now; bump on minor releases only after reading the changelog, since the public API is still pre-1.0.
*ACP 有官方 Rust SDK —— 跟 OpenZiti 不一样 —— 这里**没有** vendor 策略警告。crate（`agent-client-protocol`，Apache-2.0）是 [agentclientprotocol/rust-sdk][sdk] 维护的官方绑定。目前 pin `2.2`；minor 升级前先读 changelog —— 公开 API 还在 1.0 之前。*

[sdk]: https://github.com/agentclientprotocol/rust-sdk

## Known gaps · 已知缺口

- **Outbound `Dispatch::Request`** from the agent (e.g. fs/read) is answered with a generic `InternalError`. When the time comes to let agents call back into Synapse, plug `Builder::on_receive_request` into the `connect_with` chain.
  *agent 发出的 `Dispatch::Request`（比如 fs/read）目前用通用 `InternalError` 回应。如果要让 agent 反向调用 Synapse，把 `Builder::on_receive_request` 接进 `connect_with` 链即可。*
- **Capability advertisement** — we always send `ProtocolVersion::V1` regardless of what the agent advertised. If we start depending on v2 features (`V2SessionBuilder`, MCP-over-ACP), switch `Client::default()` to `Client::default().v2()` so negotiation fails loud when the agent is too old.
  *能力通告 —— 目前永远发送 `ProtocolVersion::V1`，不管 agent 通告什么版本。如果开始依赖 v2 特性（`V2SessionBuilder`、MCP-over-ACP），把 `Client::default()` 换成 `Client::default().v2()`，让旧 agent 上的协商直接失败而不是静默回退。*

## See also · 相关阅读

- [`src/acp/mod.rs`](../../src/acp/mod.rs) — module-level rustdoc (wire model + quick start)
- [`src/acp/client.rs`](../../src/acp/client.rs) — `AcpClient` / `AcpSessionHandle` impl
- [`src/acp/spec.rs`](../../src/acp/spec.rs) — `AcpAgentSpec` builder + presets
- [`src/acp/translate.rs`](../../src/acp/translate.rs) — `SessionUpdate` → `AgentStreamEvent`
- [`src/adk/stream/mod.rs`](../../src/adk/stream/mod.rs) — `AgentStreamEvent` reference
- [Agent Client Protocol spec][acp]
- [agent-client-protocol Rust SDK][sdk]
