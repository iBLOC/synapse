# Synapse

**Rust-based intelligent agent development framework** — 对标 LangChain / Eino，提供 LLM agent、RAG、MultiAgent、MCP、A2A、WASM 隔离等核心能力。

[![Tests](https://img.shields.io/badge/tests-274%20passing-brightgreen)]()
[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue)]()
[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE)

📖 **Full documentation:** <https://synapse-rs.github.io/synapse/> (build locally with `mdbook serve docs/`)

## ✨ 核心能力

| 能力 | 状态 | 说明 |
|------|------|------|
| Runnable / Graph / Chain | ✅ | 可组合执行原语 |
| GraphReActAgent | ✅ | Graph-based 多轮 ReAct |
| ToolsNode | ✅ | 工具调用节点 |
| DocumentTransformer + 3 chunkers | ✅ | Character / Recursive / Markdown |
| Embedding (OpenAI) + RAG (InMemory) | ✅ | 余弦相似度 + threshold 过滤 |
| MCP client (stdio) | ✅ | Model Context Protocol 集成 |
| A2A 协议 (a2a-rs v0.4) | ✅ | Agent-to-Agent 通信 (JSON-RPC + SSE) |
| WasiHttpClient (wasip2) | ✅ | WASM 隔离运行时支持 (含 SSE) |
| **End-to-end streaming** | ✅ | `AgentStreamEvent` 全链路, 取消可中断 |
| OpenAIClient + AnthropicClient | ✅ | 多 LLM provider |
| **MultiAgent 4 模式** | ✅ | Router / Orchestrator / Handoff / **Hybrid** |
| **Document 统一** | ✅ | 单一 `Document` 类型 |

## 🎯 独特能力（其他框架没有）

- **A2A 协议** — 通过官方 `a2a-rs` SDK 实现标准 A2A v1.0 (跨进程 agent 通信, JSON-RPC over SSE)
- **WASM 隔离** — `WasiHttpClient` 让 agent 在 PFC WASM 沙箱中运行，HTTP 走 wasi::http（不依赖 native reqwest）
- **End-to-end streaming** — `AgentStreamEvent` 单总线, 贯穿 LLM provider → agent → multi-agent → A2A wire
- **WASM agent ABI** — `synapse_abi_version=1` 稳定 ABI, 编译为 `wasm32-wasip2` 即可被任意 supervisor 加载
- **MultiAgent 4 模式全覆盖** — Router（LLM 路由）、Orchestrator（分解并行）、Handoff（对话切换）、Hybrid（路由 + handoff 组合）

## 🚀 快速开始

### 1. Runnable + Chain

```rust
use synapse::compose::chain::Chain;
// Coming soon: simplified API
```

### 2. GraphReActAgent（多轮 ReAct）

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

### 4. MultiAgent — Router (LLM 路由)

```rust
use synapse::adk::multiagent::RouterSupervisor;

let supervisor = RouterSupervisor::builder("supervisor", model)
    .add_agent(researcher_agent)
    .add_agent(writer_agent)
    .build();

let result = supervisor.run(input).await?;
// LLM 通过 tool call 决定调用哪个 sub-agent
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
// 然后合成最终答案
```

### 6. MultiAgent — Handoff (对话切换)

```rust
use synapse::adk::multiagent::HandoffSupervisor;

let supervisor = HandoffSupervisor::builder("supervisor")
    .add_agent("billing", billing_agent)   // 通过 event.action.transfer_to 转移
    .add_agent("refund", refund_agent)
    .add_agent("tech", tech_agent)
    .entry_point("billing")
    .max_handoffs(5)
    .build();
// Sub-agent 通过返回 action.transfer_to 决定何时 handoff
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
// 之后 LLM 合成最终答案
```

### 8. MCP (Model Context Protocol)

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn("python3", &["mcp_server.py"])?;
client.initialize().await?;
let tools = client.list_tools().await?;

// Tools 包装为 Synapse InvokableTool：
// use synapse::mcp::tool::McpToolAdapter;
```

### 9. A2A (Agent-to-Agent) — official a2a-rs SDK

```rust
// Host a Synapse agent as an A2A server:
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
// Call a remote A2A peer:
let client = synapse::a2a::client::A2AClient::new("https://peer.example.com").await?;
let stream = client.send_message_stream(synapse::schema::Message::user("hi")).await?;
while let Some(event) = stream.next().await { /* ... */ }
```

## 🏗️ 架构

```
┌─────────────────────────────────────────────┐
│  Adk (Agent Development Kit)                │
│   ├─ Agent trait                            │
│   ├─ ChatModelAgent (LLM agent + tools)     │
│   ├─ GraphReActAgent (graph-based)          │
│   └─ MultiAgent                             │
│      ├─ RouterSupervisor (LLM-driven)       │
│      ├─ OrchestratorAgent (parallel)        │
│      ├─ HandoffSupervisor (transfer)        │
│      └─ HybridSupervisor (router+handoff)   │
├─────────────────────────────────────────────┤
│  Components                                 │
│   ├─ Model: OpenAI, Anthropic               │
│   ├─ Tool: BaseTool, InvokableTool          │
│   ├─ Document: parser, transformer          │
│   │   ├─ CharacterTextSplitter              │
│   │   ├─ RecursiveCharacterTextSplitter     │
│   │   └─ MarkdownHeaderSplitter             │
│   ├─ Embedding: OpenAI                      │
│   ├─ Indexer: InMemory                      │
│   └─ Retriever: InMemory (cosine)           │
├─────────────────────────────────────────────┤
│  Transport (cross-platform)                 │
│   ├─ HttpClient trait                       │
│   ├─ ReqwestHttpClient (native)             │
│   └─ WasiHttpClient (wasip2)                │
├─────────────────────────────────────────────┤
│  MCP (Model Context Protocol)               │
│   └─ StdioMcpClient + McpToolAdapter       │
├─────────────────────────────────────────────┤
│  Protocol                                   │
│   └─ A2A Adapter                            │
└─────────────────────────────────────────────┘
```

## 🧪 测试

```bash
cargo test
```

**157 tests passing**（基础 87 + MultiAgent 26 + Document 9 + Transformer + 整合 22 + 整合 11）。

**v0.2 增量**: +14 A2A 集成测试, 总计 **274 tests passing**。

## 🆕 v0.2 增量能力 (2026-07-01)

- **End-to-end streaming** — `AgentStreamEvent` 单总线, 贯穿 LLM provider → agent → multi-agent → A2A wire
- **a2a-rs SDK 集成** — 完全使用官方 SDK (`a2a-lf` 0.3, `a2a-server-lf` 0.4, `a2a-client-lf` 0.2)
- **WASM agent ABI v1** — `synapse_abi_version=1` 稳定 ABI, 编译为 `wasm32-wasip2` 即可被任意 supervisor 加载
- **MSRV 1.85** (从 1.75 升级)
- **WasiHttpClient SSE** — WASM 端流式 LLM 调用

详细文档：
- `docs/wasm-agent-abi.md` — WASM 协议
- `docs/wasm-compilation-guide.md` — 工程实践
- `templates/agent/` — copy-and-edit 的 WASM agent 脚手架
- `docs/superpowers/specs/2026-07-01-v0.2-architecture.md` — 架构 spec
- `docs/superpowers/specs/2026-07-01-v0.2-status.md` — 实施状态

## 📦 编译目标

- **Native** (default): 完整功能 (axum + reqwest + a2a-rs)
- **WASM** (`--target wasm32-wasip2`): `WasiHttpClient` 让 agent 在 WASM 沙箱中运行（通过 host function 调用 HTTP）。A2A server 在 native, agent WASM 通过 `synapse:host/a2a-call` import 调用外部 peer
- **Agent template** (`templates/agent/`): 用户级 entrypoint, `cargo build --release --target wasm32-wasip2` 产出可加载的 `.wasm`

## 📚 文档

完整使用文档：<https://synapse-rs.github.io/synapse/>

本地构建：
```bash
cargo install mdbook
cd docs && mdbook serve
```

设计 Specs：`docs/superpowers/specs/`

## 📝 License

[Apache License 2.0](LICENSE)
