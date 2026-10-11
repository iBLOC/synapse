---
title: ACP Client
---

# ACP 客户端

`synapse::acp` 模块驱动通过 [Agent Client Protocol][acp] 与 Synapse 对话的外部编码 agent（opencode、claude-code-acp、codex-acp …），走 stdio JSON-RPC，并把输出映射为常规的 Synapse [`AgentStreamEvent`](../../src/adk/stream/mod.rs) 流。

> 目前仅做 **client-only**：Synapse 主动 *连接* 外部 ACP agent 并消费其响应。在 Synapse 内部实现 ACP `Agent` 角色不在本模块范围内。

## 协议模型

```
┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
│ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
│ AcpClient      │                        │ (external subprocess)   │
└────────────────┘                        └────────────────────────┘
```

> `AcpSession` 是 `AcpSessionHandle` 的公开 re-export 别名（见 [`src/acp/session.rs`](../../src/acp/session.rs)）。两个名字指向同一类型；本文为简洁起见用 `AcpSession`。

SDK 由 [`AcpAgent`][sdk-agent] 负责 spawn 子进程；我们不直接碰 `tokio::process::Command`。每次 [`AcpClient::session()`](../../src/acp/client.rs) 返回的都是借用句柄；底层 `agent-client-protocol` session 在 `prompt()` 内部按调用逐次新建（同一 stdio transport 上每个 prompt 开一个 session）。

## 为什么用官方 SDK

四个正确性陷阱由官方 SDK 处理，我们不再手写 JSON-RPC 客户端：

   1. 协议版本协商 —— SDK 根据 `InitializeResponse.protocol_version` 自动升降级。手写客户端会卡在 `V1`，遇到 v2 agent 就静默坏掉。
   2. 请求/响应关联 —— `SentRequest::block_task().await` 加上 SDK 内部的请求 ID 簿记消除了"哪个响应属于哪个请求"的竞态。手写客户端几乎都会在这上面栽。
   3. `$/cancelRequest` 语义 —— UI 取消能在前端有代理的情况下正确回路。手写客户端经常忘记发 JSON-RPC cancel 通知，导致子进程停不掉。
   4. stdio framing —— `Stdio` 帮我们处理 `Content-Length` header vs line-delimited frame 两种编码，`AsyncBufReadExt::lines()` 只能处理后者。

SDK 还免费赠送这些前向兼容能力：

  - `Conductor` / `Proxy` 角色已为多 agent fan-out 备好；shadow-desktop 同时驱动 opencode + claude-code-acp 时不用改连接层。
  - `with_mcp_server` 允许 Synapse 把自己的 MCP server 注入到 session —— agent 调用我们的工具不需要多开一个端口。
  - `SessionUpdate` 是 `#[non_exhaustive]`，迫使 `translate_update` 走 `serde_json` 往返；新 schema variant 会落进 `_ => Vec::new()` 分支而不是直接编不过。

## 构建与 feature 开关

默认 **off**，仅在 non-wasm 目标编译 —— 上游 SDK 不在 `target_arch = "wasm32"` 上导出 `AcpAgent` / `Stdio`。

```bash
cargo build  -p synapse --features acp
cargo test   -p synapse --features acp --lib acp
```

## 快速开始

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

## 公开 API

| 项目 | 类型 | 用途 |
|------|------|------|
| `AcpClient` | `struct` (cheap-clone via `Arc`) | 持有一条外部 ACP agent 连接的客户端 |
| `AcpClient::connect(spec)` | `async fn` | 起子进程、跑握手、起 dispatch 任务 |
| `AcpClient::session()` | `fn -> AcpSessionHandle<'_>` | 借用句柄用于发起 prompt |
| `AcpSessionHandle::prompt(text)` *(a.k.a. `AcpSession::prompt`)* | `async fn -> Result<StreamReader<AgentStreamEvent>, AcpError>` | 发一个 prompt，流式返回事件 |
| `AcpAgentSpec` | `struct` (with builder) | 描述子进程的纯数据 |
| `AcpAgentSpec::opencode()` | preset | 调用 opencode |
| `AcpAgentSpec::claude_code()` | preset | 调用 claude-code |
| `AcpAgentSpec::codex()` | preset | 调用 codex |
| `AcpAgentSpec::new(name, cmd)` | builder | 自定义命令 |
| `AcpAgentSpec::arg(...)` / `.args(...)` / `.env(k, v)` | builder | 扩展 spec |
| `AcpError::Sdk` | enum variant | SDK 底层错误 |
| `AcpError::Cancelled` | enum variant | 调用方通过 `CancelHandle` 取消 |

## 自定义 agent

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

builder 把 args / env 叠加在继承的环境变量上 —— 你只需声明 agent 真正需要的变量。

## 事件映射

| ACP 来源 | `AgentStreamEvent` |
|------|------|
| `SessionUpdate::Pending` / `InProgress` (tool) | `ToolStart { name, input, agent }` |
| `SessionUpdate::Completed` / `Failed` (tool) | `ToolEnd { name, output, status, agent }` |
| `AgentMessageChunk` | `TokenDelta { text, agent }` |
| `StopReason::EndTurn` | `Final { agent, message }` |

九个单测覆盖 `AcpAgentSpec` builder、`SessionUpdate → AgentStreamEvent` 翻译器、以及 `SessionNotification` envelope 解析（envelope / 裸 update / 不可解析 / 往返）。

## 升级路径

调用方只依赖 `AcpClient` / `AcpSession`（`AcpSessionHandle`）/`AcpAgentSpec` / `AcpError`。线协议类型（`SessionUpdate`、`ToolCallStatus` …）被 `translate.rs` 隔离，SDK 未来大版本（v3）改名/重构只是翻译器局部改动，公开 API 不动。

## SDK 注意事项

ACP 有官方 Rust SDK —— 跟 OpenZiti 不一样 —— 这里**没有** vendor 策略警告。crate（`agent-client-protocol`，Apache-2.0）是 [agentclientprotocol/rust-sdk][sdk] 维护的官方绑定。目前 pin `2.2`；minor 升级前先读 changelog —— 公开 API 还在 1.0 之前。

## 已知缺口

  - agent 发出的 `Dispatch::Request`（比如 fs/read）目前用通用 `InternalError` 回应。如果要让 agent 反向调用 Synapse，把 `Builder::on_receive_request` 接进 `connect_with` 链即可。
  - 能力通告 —— 目前永远发送 `ProtocolVersion::V1`，不管 agent 通告什么版本。如果开始依赖 v2 特性（`V2SessionBuilder`、MCP-over-ACP），把 `Client::default()` 换成 `Client::default().v2()`，让旧 agent 上的协商直接失败而不是静默回退。

## 相关阅读
