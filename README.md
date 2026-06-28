# Synapse

**Rust-based intelligent agent development framework** — 对标 LangChain / Eino，提供 LLM agent、RAG、MultiAgent、MCP、A2A、WASM 隔离等核心能力。

[![Tests](https://img.shields.io/badge/tests-157%20passing-brightgreen)]()
[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue)]()
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
| A2A 协议 | ✅ | Agent-to-Agent 通信 |
| WasiHttpClient (wasip2) | ✅ | WASM 隔离运行时支持 |
| OpenAIClient + AnthropicClient | ✅ | 多 LLM provider |
| **MultiAgent 4 模式** | ✅ | Router / Orchestrator / Handoff / **Hybrid** |
| **Document 统一** | ✅ | 单一 `Document` 类型 |

## 🎯 独特能力（其他框架没有）

- **A2A 协议** — 跨进程 agent 通信
- **WASM 隔离** — `WasiHttpClient` 让 agent 在 PFC WASM 沙箱中运行，HTTP 走 wasi::http（不依赖 native reqwest）
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

### 9. A2A (Agent-to-Agent)

```rust
use synapse::protocol::a2a_adapter::A2AAdapter;
use synapse::transport::ReqwestHttpClient;

let a2a = A2AAdapter::new(Arc::new(ReqwestHttpClient::new()));
// Send A2A messages between agents
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

## 📦 编译目标

- **Native** (default): 完整功能
- **WASM** (`--target wasm32-wasip2`): `WasiHttpClient` 让 agent 在 WASM 沙箱中运行（通过 host function 调用 HTTP）

## 🌐 相关项目

- **神经突触 Synapse**（本项目）— Agent 开发工具包
- **神经元 Neuron** — LLM 运行时
- **前额叶 PFC** — 多 agent 编排 + WASM 隔离运行时

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
