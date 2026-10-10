# Synapse

**Rust-based intelligent agent development framework** — 对标 LangChain / Eino，提供 LLM agent、RAG、MultiAgent、MCP、A2A、WASM 隔离、ACP、lobe 等核心能力。
*Rust intelligent-agent development framework — provides LLM agents, RAG, MultiAgent, MCP, A2A, WASM sandboxing, ACP, lobe lifecycle glue, and more.*

[![Tests](https://img.shields.io/badge/tests-292%20passing-brightgreen)]()
[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue)]()
[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE)

📖 **完整文档 Full documentation:** <https://ibloc.github.io/synapse/> · 本地构建 *build locally*: `cd docs && mdbook serve`

---

## ✨ 核心能力 / Core Capabilities

| 能力 / Capability | 状态 / Status | 说明 / Description |
|------|:------:|------|
| Runnable / Graph / Chain | ✅ | 可组合执行原语 / Composable execution primitives |
| GraphReActAgent | ✅ | Graph-based 多轮 ReAct / Multi-turn graph-based ReAct |
| ToolsNode | ✅ | 工具调用节点 / Tool-call node |
| DocumentTransformer + 3 chunkers | ✅ | Character / Recursive / Markdown |
| Embedding (OpenAI) + RAG (InMemory) | ✅ | 余弦相似度 + threshold 过滤 / Cosine similarity + threshold |
| **MCP client** (stdio) | ✅ | Model Context Protocol — client role |
| **MCP server** | ✅ | 把 Synapse 工具暴露为 MCP server / Expose Synapse tools as an MCP server |
| **A2A 协议 / A2A protocol** (a2a-rs v0.4) | ✅ | Agent-to-Agent 通信 / Communication (JSON-RPC + SSE) |
| **A2A metadata wire-forwarding** | ✅ | `__encephalon_meta_v1__` 通道 / Channel |
| WasiHttpClient (wasip2) | ✅ | WASM 隔离运行时支持 (含 SSE) / WASM sandbox runtime (incl. SSE) |
| **End-to-end streaming** | ✅ | `AgentStreamEvent` 全链路 / End-to-end pipeline, 取消可中断 / cancellation-aware |
| OpenAIClient + AnthropicClient | ✅ | 多 LLM provider / Multi-LLM provider |
| **MultiAgent 4 模式 / 4 modes** | ✅ | Router / Orchestrator / Handoff / **Hybrid** |
| **Document 统一 / Unified `Document`** | ✅ | 单一类型 / Single type |
| **ACP client** *(feature = `acp`)* | ✅ | 驱动 opencode / claude-code-acp / codex-acp 走 stdio JSON-RPC |
| **`transport::ziti`** *(feature = `ziti`)* | ✅ | 零信任 A2A over OpenZiti dark services |
| **`synapse::lobe`** (v0.4) | ✅ | Stateless skill lifecycle glue + A2A server scaffolding |
| **Audio providers** (ASR + TTS) | ✅ | `AsrProvider` / `TtsProvider` 走 OpenAI-compatible endpoint |
| **Provider catalog** | ✅ | 19 预设 + 双标准 list_models (OpenAI / Claude) / Dual-standard model listing |
| Retry policy | ✅ | 4 模式 backoff / 4-mode backoff |

## 🎯 独特能力 / Differentiators

> 其他框架没有的 / Things other frameworks don't have.

- **A2A 协议 / A2A protocol** — 通过官方 `a2a-rs` SDK 实现标准 A2A v1.0 (跨进程 agent 通信, JSON-RPC over SSE) / Cross-process agent communication via the official a2a-rs SDK.
- **WASM 隔离 / WASM sandboxing** — `WasiHttpClient` 让 agent 在 PFC WASM 沙箱中运行，HTTP 走 wasi::http（不依赖 native reqwest）/ Agents run inside a PFC WASM sandbox; HTTP goes through `wasi::http` (no native reqwest dependency).
- **End-to-end streaming** — `AgentStreamEvent` 单总线, 贯穿 LLM provider → agent → multi-agent → A2A wire / Single bus from LLM provider through agent, multi-agent, and A2A wire.
- **WASM agent ABI v1** — `synapse_abi_version=1` 稳定 ABI, 编译为 `wasm32-wasip2` 即可被任意 supervisor 加载 / Stable ABI; compile to `wasm32-wasip2` and any supervisor can load it.
- **MultiAgent 4 模式全覆盖 / 4-mode coverage** — Router（LLM 路由 / LLM routing）、Orchestrator（分解并行 / decompose + parallel）、Handoff（对话切换 / conversation transfer）、Hybrid（路由 + handoff 组合 / router + handoff composite）.
- **ACP client** — 复用官方 `agent-client-protocol` 2.2 SDK，把 opencode / claude-code-acp / codex-acp 的 stdio JSON-RPC 流接成普通 `AgentStreamEvent`，让 Synapse 复用现有 agent 抽象驱动任何 ACP-speaking 编码 agent / Reuse the official `agent-client-protocol` 2.2 SDK; surface opencode / claude-code-acp / codex-acp stdio JSON-RPC as ordinary `AgentStreamEvent`s so Synapse can drive any ACP-speaking coding agent through its existing agent abstractions.
- **`transport::ziti`** — 零信任 A2A over OpenZiti dark services（替代明文 mTLS over LAN）/ Zero-trust A2A over OpenZiti dark services (alternative to plaintext mTLS over LAN). 公网 IP 完全不可见 / Public IP is invisible.
- **`synapse::lobe`** — 10 行代码搭一个 stateless skill executor + A2A server，注册到 encephalon / 10-line boilerplate for a stateless skill executor with an A2A server, registered with encephalon.

---

## 🚀 快速开始 / Quick Start

### 1. Runnable + Chain

```rust
use synapse::compose::chain::Chain;
// Coming soon: simplified API
```

### 2. GraphReActAgent（多轮 ReAct / Multi-turn ReAct）

```rust
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "your-key"));
let agent = GraphReActAgent::builder()
    .name("assistant")
    .with_model(model)
    .with_tool(Arc::new(my_tool))
    .max_steps(5)
    .build()?;
```

### 3. RAG + Chunking

```rust
use synapse::components::transformer::RecursiveCharacterTextSplitter;
use synapse::components::rag::InMemoryIndexer;
use synapse::components::embedding::OpenAIEmbedding;

let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
let chunks = splitter.transform(long_docs).await?;

let embedder = Arc::new(OpenAIEmbedding::new("text-embedding-3-small", "key"));
let indexer = InMemoryIndexer::new(store, embedder);
indexer.index(chunks).await?;
```

### 4. MultiAgent — Router (LLM 路由 / LLM-driven routing)

```rust
use synapse::adk::multiagent::RouterSupervisor;

let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .build();

let result = supervisor.run(input).await?;
// LLM 通过 tool call 决定调用哪个 sub-agent / The LLM picks the sub-agent via tool calls.
```

### 5. MultiAgent — Orchestrator (并行 dispatch + synthesize)

```rust
use synapse::adk::multiagent::OrchestratorAgent;

let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .add_agent(researcher_agent)
    .add_agent(analyst_agent)
    .add_agent(writer_agent)
    .build();
// LLM 一次 dispatch 多个 sub-agent (parallel tool calls)，
// 然后合成最终答案 / Dispatch several sub-agents in parallel tool calls,
// then synthesize the final answer.
```

### 6. MultiAgent — Handoff (对话切换 / Conversation transfer)

```rust
use synapse::adk::multiagent::HandoffSupervisor;

let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)   // 通过 event.action.transfer_to 转移
    .add_agent("refund", refund_agent)
    .add_agent("tech", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
// Sub-agent 通过返回 action.transfer_to 决定何时 handoff / Sub-agents decide when to hand off
// by returning action.transfer_to.
```

### 7. MultiAgent — Hybrid (LLM 路由 + handoff)

```rust
use synapse::adk::multiagent::HybridSupervisor;

let supervisor = HybridSupervisor::builder("hybrid", model)
    .add_agent("billing", billing_agent)
    .add_agent("refund", refund_agent)
    .max_handoffs(5)
    .build();
// LLM 先路由到 billing，billing 可 handoff 到 refund，
// 之后 LLM 合成最终答案 / LLM routes to billing, billing can hand off to refund,
// then LLM synthesizes the final answer.
```

### 8. MCP — Client (stdio) / Client role over stdio

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn("python3", &["mcp_server.py"])?;
client.initialize().await?;
let tools = client.list_tools().await?;

// Tools 包装为 Synapse InvokableTool：
// Wrap tools as a Synapse InvokableTool:
// use synapse::mcp::tool::McpToolAdapter;
```

### 9. MCP — Server / Server role (new)

```rust
use synapse::mcp::server::McpServer;
// 把 Synapse 工具挂到 MCP server 上，让外部 MCP client (Claude Desktop 等) 调用
// Mount Synapse tools on an MCP server so external MCP clients (Claude Desktop, etc.)
// can call them.
```

### 10. A2A — Host a Synapse agent / Host a Synapse agent as an A2A server

```rust
use synapse::a2a::server::A2AServerBuilder;
use synapse::a2a::card::AgentCard;
use std::sync::Arc;

let agent: Arc<dyn synapse::adk::agent::Agent> = todo!();
let card = AgentCard::new("my-agent", "A helpful assistant");
A2AServerBuilder::new(agent, card)
    .bind("0.0.0.0:8080".parse()?)
    .serve().await?;
```

```rust
// Call a remote A2A peer / 调用远端 A2A peer:
let client = synapse::a2a::client::A2AClient::new("https://peer.example.com").await?;
let stream = client.send_message_stream(synapse::schema::Message::user("hi")).await?;
while let Some(event) = stream.next().await { /* ... */ }
```

### 11. ACP client — drive an external coding agent *(feature = `acp`)*

```rust,no_run
# #[cfg(feature = "acp")]
# async fn run() -> Result<(), synapse::acp::AcpError> {
use synapse::acp::{AcpAgentSpec, AcpClient};

// Talk to `opencode --acp` / `claude-agent-acp` / `codex-acp` (preset helpers).
let client = AcpClient::connect(AcpAgentSpec::opencode()).await?;

// Send one prompt; consume the stream of `AgentStreamEvent`s.
let mut events = client.session().prompt("Summarise this file").await?;
while let Some(ev) = events.next().await {
    // ToolStart / ToolEnd / TokenDelta / FinalAnswer — same as a native agent.
}
# Ok(()) }
```

详见 / See [`src/acp/mod.rs`](src/acp/mod.rs) 顶部的模块文档 (Wire model + 为什么用官方 SDK + 已知缺口)。

### 12. `synapse::lobe` — stateless skill + A2A in 10 lines

```rust,no_run
use synapse::lobe::{LobeApp, StatelessSkill, SkillContext, SkillOutput};

struct EchoSkill;

#[async_trait::async_trait]
impl StatelessSkill for EchoSkill {
    type Input = String;
    type Output = String;
    async fn run(&self, input: Self::Input, _ctx: &SkillContext) -> SkillOutput<Self::Output> {
        SkillOutput::ok(input)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    LobeApp::new("echo", "Echo skill")
        .skill(Arc::new(EchoSkill))
        .serve("0.0.0.0:9101").await?;
    Ok(())
}
```

自动获得：AgentCard 注册到 encephalon + axum A2A server + `StatelessSkill` lifecycle glue。
*You automatically get: AgentCard registration with encephalon + an axum A2A server + StatelessSkill lifecycle glue.*

---

## 🏗️ 架构 / Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Adk (Agent Development Kit)                                │
│   ├─ Agent trait                                            │
│   ├─ ChatModelAgent (LLM agent + tools)                     │
│   ├─ GraphReActAgent (graph-based)                          │
│   └─ MultiAgent                                             │
│      ├─ RouterSupervisor (LLM-driven)                       │
│      ├─ OrchestratorAgent (parallel)                        │
│      ├─ HandoffSupervisor (transfer)                        │
│      └─ HybridSupervisor (router+handoff)                   │
├─────────────────────────────────────────────────────────────┤
│  Components                                                 │
│   ├─ Model: OpenAI, Anthropic, custom                       │
│   ├─ Tool: BaseTool, InvokableTool, StreamableTool          │
│   ├─ Document: parser, transformer                          │
│   │   ├─ CharacterTextSplitter                              │
│   │   ├─ RecursiveCharacterTextSplitter                     │
│   │   └─ MarkdownHeaderSplitter                             │
│   ├─ Embedding: OpenAI                                      │
│   ├─ Indexer: InMemory (pgvector 等规划中 / planned)         │
│   └─ Retriever: InMemory (cosine)                           │
├─────────────────────────────────────────────────────────────┤
│  lobe (v0.4) — stateless skill lifecycle glue               │
│   ├─ StatelessSkill / SkillContext / SkillOutput traits     │
│   ├─ ProviderSet (LLM + TTS + ASR + Emotion bundle)        │
│   ├─ LobeApp (10-line server + encephalon registration)     │
│   ├─ lobe::metrics, lobe::history (HistorySummarizer)       │
│   └─ lobe::media_pipeline (audio/video ingest + emit)       │
├─────────────────────────────────────────────────────────────┤
│  acp (feature = "acp") — drive external ACP agents          │
│   ├─ AcpAgentSpec (opencode / claude_code / codex / custom) │
│   ├─ AcpClient (spawn + initialize handshake + dispatch)    │
│   ├─ AcpSession (per-prompt stream of AgentStreamEvent)     │
│   └─ translate.rs (SessionUpdate → AgentStreamEvent)        │
├─────────────────────────────────────────────────────────────┤
│  mcp — Model Context Protocol                               │
│   ├─ StdioMcpClient (client role)                           │
│   ├─ HttpMcpClient (HTTP transport)                         │
│   ├─ McpServer (server role — expose Synapse tools)         │
│   └─ McpToolAdapter (wrap MCP tool → InvokableTool)         │
├─────────────────────────────────────────────────────────────┤
│  providers — LLMProvider + audio providers                  │
│   ├─ OpenAIProvider / AnthropicProvider / Reqwest           │
│   ├─ providers::catalog (19 presets, dual-standard list)    │
│   ├─ providers::retry (4-mode backoff)                      │
│   └─ providers::audio::{AsrProvider, TtsProvider}           │
├─────────────────────────────────────────────────────────────┤
│  Protocol                                                   │
│   ├─ a2a — A2A Adapter + metadata wire-forwarding           │
│   │        (__encephalon_meta_v1__ packed channel)          │
│   └─ HybridRAG / Rerank / Retriever                         │
├─────────────────────────────────────────────────────────────┤
│  Transport (cross-platform HTTP)                            │
│   ├─ HttpClient trait                                       │
│   ├─ ReqwestHttpClient (native)                             │
│   ├─ WasiHttpClient (wasip2)                                │
│   └─ transport::ziti (feature = "ziti", OpenZiti dark mode)  │
│        ├─ ZitiGateway / ZitiListener / ZitiStream           │
│        ├─ ZitiHttpClient / ConnectionPool                   │
│        └─ serve(router, identity, service) one-liner        │
├─────────────────────────────────────────────────────────────┤
│  Cross-cutting                                              │
│   ├─ session / session events (typed log + replay)          │
│   ├─ skills (namespace + registration)                      │
│   ├─ decision (memory store) / memory helpers               │
│   ├─ events / callbacks / channels                          │
│   ├─ thought (chain-of-thought helpers)                     │
│   ├─ scheduler / task                                       │
│   ├─ structured (typed lambda outputs)                      │
│   └─ trace_context (distributed tracing)                    │
└─────────────────────────────────────────────────────────────┘
```

---

## 🧪 测试 / Tests

```bash
cargo test                           # default features — 292 tests
cargo test --features acp            # +9 ACP client tests
cargo test --features ziti           # +Ziti transport tests
cargo test --all-features            # everything
```

**301 tests passing with `--features acp`, 292 default**（截至 2026-10-10，含 9 个 ACP 单测、9 个 synapse::lobe 集成测试、双标准模型 list 单测等）。
*As of 2026-10-10: 301 tests passing with `--features acp`, 292 default (incl. 9 ACP unit tests, 9 synapse::lobe integration tests, dual-standard model listing tests).*

```bash
cargo test --lib providers          # provider catalog + retry policy
cargo test --lib acp --features acp # ACP client + SessionUpdate translator
cargo test --lib lobe               # StatelessSkill + LobeApp
cargo test --lib transport          # HttpClient trait + native/wasi impls
```

---

## 📦 编译目标 / Build Targets

### 默认 / Default

```bash
cargo build                           # 完整功能 / full native build (axum + reqwest + a2a-rs)
cargo build --release                 # 发布构建 / release build
```

### WASM (`wasm32-wasip2`)

```bash
cargo build --release --target wasm32-wasip2
```

`WasiHttpClient` 让 agent 在 WASM 沙箱中运行（通过 host function 调用 HTTP）。A2A server 在 native 端；agent WASM 通过 `synapse:host/a2a-call` import 调用外部 peer。
*`WasiHttpClient` lets the agent run inside a WASM sandbox (HTTP goes through host functions). The A2A server stays native; agent WASM calls external peers through the `synapse:host/a2a-call` import.*

### Agent 模板 / Agent template (`templates/agent/`)

```bash
cd templates/agent
cargo build --release --target wasm32-wasip2  # 产出可加载的 .wasm / produces a loadable .wasm
```

### Feature flags

| Feature | 用途 / Purpose | 依赖 / Depends on | 默认 / Default |
|---------|------|------|:---:|
| *(none)* | 默认 native 构建，涵盖 a2a/adk/components/lobe/mcp/providers/transport (不含 acp / ziti) | — | ✅ |
| `acp` | ACP client（驱动外部编码 agent）/ Drive external ACP agents over stdio | `agent-client-protocol` 2.2 | ❌ |
| `ziti` | `transport::ziti` 零信任 dark service | `ziti-sdk` 0.3 (third-party) | ❌ |
| `audio` | ASR / TTS 启用 / Enable ASR / TTS providers | `reqwest` multipart | ❌ |

> ACP 与 ziti 都不在 wasm32 上编译 — 上游 SDK 不支持。*ACP and ziti do not compile on `wasm32` — the upstream SDKs do not support it.*

---

## 🆕 近期增量 / Recent Changes

### v0.5 — ACP + Lobe + Audio (2026-09)

- **`synapse::acp` client** — 官方 `agent-client-protocol` 2.2 SDK 驱动 opencode / claude-code-acp / codex-acp；复用 Synapse 现有 `AgentStreamEvent` 总线，四个常见 JSON-RPC 陷阱（协议协商 / 请求关联 / `$/cancelRequest` / stdio framing）一律交给 SDK 兜底。
  *Official `agent-client-protocol` 2.2 SDK driving opencode / claude-code-acp / codex-acp; reuses Synapse's existing `AgentStreamEvent` bus and delegates the four common JSON-RPC traps (protocol negotiation / request correlation / `$/cancelRequest` / stdio framing) to the upstream SDK.*
- **`synapse::lobe`** — v0.4 stateless skill lifecycle：traits (StatelessSkill / SkillContext / SkillOutput / EmotionProvider / LobeEnv)、LobeApp 10 行 server 模板、ProviderSet bundle、HistorySummarizer、metrics。
  *v0.4 stateless skill lifecycle: traits (StatelessSkill / SkillContext / SkillOutput / EmotionProvider / LobeEnv), LobeApp 10-line server scaffold, ProviderSet bundle, HistorySummarizer, metrics.*
- **Audio providers** — `AsrProvider` / `TtsProvider` 走 OpenAI-compatible endpoint；替代各 lobe 里重复写的 HTTP 样板。
  *`AsrProvider` / `TtsProvider` over OpenAI-compatible endpoints; replaces HTTP boilerplate scattered across lobes.*
- **`transport::ziti`** — 零信任 A2A over OpenZiti dark services（公网 IP 完全不可见）；feature gated。
  *Zero-trust A2A over OpenZiti dark services (public IP invisible); feature gated.*

### v0.4 — A2A + WASM-A2A bridge (2026-07)

- **A2A metadata wire-forwarding** — encephalon 把 `metadata['name']` 打包成 `__encephalon_meta_v1__` 透传到 `into_a2a_message`，Synapse 端用 `decode_metadata_from_name` 还原。
  *encephalon packs `metadata['name']` into `__encephalon_meta_v1__` and forwards it through `into_a2a_message`; Synapse decodes with `decode_metadata_from_name`.*
- **WASM-A2A sync bridge** — `call_entry` 现在真的从 linear memory 读 N 字节（之前返回 placeholder），真实 WASM agent 端到端打通。
  *`call_entry` now actually reads N bytes from linear memory (was returning a placeholder); real WASM agents reach the A2A server end-to-end.*

### v0.3 — End-to-end streaming + a2a-rs SDK (2026-07)

- **`AgentStreamEvent`** — 单总线贯穿 LLM provider → agent → multi-agent → A2A wire；取消可中断。
  *Single bus from LLM provider through agent, multi-agent, and A2A wire; cancellation-aware.*
- **a2a-rs SDK 集成** — 官方 `a2a-lf` 0.3 / `a2a-server-lf` 0.4 / `a2a-client-lf` 0.2。
  *Official `a2a-lf` 0.3 / `a2a-server-lf` 0.4 / `a2a-client-lf` 0.2 integration.*
- **WASM agent ABI v1** — `synapse_abi_version=1` 稳定 ABI；`cargo build --target wasm32-wasip2` 即被任意 supervisor 加载。
  *Stable `synapse_abi_version=1` ABI; compile to `wasm32-wasip2` and any supervisor can load it.*
- **MSRV 1.85**（从 1.75 升级 / bumped from 1.75）。

### v0.2 — Provider catalog + Provider config + Dual-standard listing

- `providers::catalog` 19 预设（OpenAI / Anthropic / DeepSeek / 智谱 / 通义 / Ollama / 本地模型 …），OpenAI + Claude 双标准 list_models。
  *19 presets (OpenAI / Anthropic / DeepSeek / Zhipu / Qwen / Ollama / local neuron …) with dual-standard list_models.*
- `providers::config` 加密 schema store + `all_schemas()`。
  *Encrypted schema store + `all_schemas()` for AI agent runtime.*

### v0.1 — Initial release

- Runnable / Chain / GraphReActAgent / ToolsNode / Document / Embedding / RAG / MCP client (stdio) / A2A / WasiHttpClient / 4-mode MultiAgent.

---

## 🛠️ SDK 选型说明 / SDK Choice Rationale

### ACP — 官方 SDK (`agent-client-protocol` 2.2) / Official SDK

四个正确性陷阱由上游 SDK 处理：协议版本协商、请求/响应 ID 关联、`$/cancelRequest` 取消语义、stdio framing。ACP 官方有 Rust SDK — 不存在 vendor 风险。
*The four correctness traps (protocol version negotiation, request/response correlation, `$/cancelRequest` cancel semantics, stdio framing) are handled by the upstream SDK. ACP ships an official Rust SDK — no vendor-strategy caveat.*

### OpenZiti — 第三方 SDK (`ziti-sdk` 0.3, jaschadub/ziti-rust)

⚠️ OpenZiti 项目没有官方 Rust 绑定；crates.io 上的 `jaschadub/ziti-rust` 是社区维护。**生产建议 vendor 化** —— fork 到组织命名空间并用 `[patch.crates-io]` 锁定，防止上游停滞拖垮 encephalon。
*⚠️ OpenZiti has no official Rust binding; `jaschadub/ziti-rust` on crates.io is community-maintained. **Vendor strategy recommended for production** — fork into the org namespace and pin via `[patch.crates-io]` so a stuck upstream doesn't take encephalon with it.*

---

## 📚 文档 / Documentation

完整使用文档 / Full docs: <https://ibloc.github.io/synapse/>

本地构建 / Build locally:
```bash
cargo install mdbook
cd docs && mdbook serve
```

设计 Specs / Design specs: `docs/superpowers/specs/`
概念文档 / Concept docs: [`docs/concepts/`](docs/concepts/) — [a2a](docs/concepts/a2a.md) · [acp](docs/concepts/acp.md) · [lobe](docs/concepts/lobe.md) · [ziti](docs/concepts/ziti.md) · [architecture](docs/concepts/architecture.md) · [mcp](docs/concepts/mcp.md) · [wasm](docs/concepts/wasm.md) · [multiagent](docs/concepts/multiagent.md) · …
AGENTS.md / 模块全景 / Module map: [`AGENTS.md`](AGENTS.md)

---

## 🤝 贡献 / Contributing

见 [`CONTRIBUTING.md`](CONTRIBUTING.md)。欢迎 PR — 至少跑 `cargo test --all-features` + `cargo clippy --all-features -- -D warnings`。
*See [`CONTRIBUTING.md`](CONTRIBUTING.md). PRs welcome — please run `cargo test --all-features` and `cargo clippy --all-features -- -D warnings`.*

---

## 📝 License

[Apache License 2.0](LICENSE)
