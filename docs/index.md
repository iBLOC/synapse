---
title: What is Synapse?
description: Rust-based intelligent agent development framework
layout: home

hero:
  name: Synapse
  text: Rust Agent Framework
  tagline: ReAct · RAG · MultiAgent · MCP · A2A · WASM
  actions:
    - theme: brand
      text: Quick Start
      link: /guide/quick-start
    - theme: alt
      text: GitHub
      link: https://github.com/iBLOC/synapse

features:
  - title: 🚀 Production-Ready Core
    details: Runnable, Graph, Chain, ReAct Agent. 292 tests passing. Apache 2.0.
  - title: 🤖 4 MultiAgent Patterns
    details: Router, Orchestrator, Handoff, Hybrid. Nested composition. LLM-driven and rule-based.
  - title: 📚 RAG Out of the Box
    details: 4 chunkers (Character/Recursive/Markdown/Token), OpenAI embedding, in-memory vector store.
  - title: 🔌 MCP + A2A
    details: Model Context Protocol (stdio + HTTP + Server). Agent-to-Agent protocol. Cross-process.
  - title: 📝 Session Event Log
    details: Append-only event persistence. FileEventStore with JSON Lines format.
  - title: 🌊 Waterfall Events
    details: 5 dispatch modes (emit/parallel/serial/bail/waterfall). Interceptor pattern.
  - title: 🔄 LLM Retry Policy
    details: Exponential backoff with jitter. Configurable retryable error codes.
  - title: 🏷️ Tool Namespace
    details: Scoped skill registration. MCP-style qualified names (mcp__server__tool).
  - title: 🌐 WASM Sandbox Ready
    details: Cross-platform HttpClient. Run agents in wasi sandboxes (PFC runtime).
  - title: 🛡️ Type-Safe Rust
    details: Async-native, no Python runtime, type-safe agent composition.
---

## Why Synapse? · 为什么选择 Synapse？

| Capability / 能力 | Synapse | LangChain | Eino |
|------------|---------|-----------|------|
| Runnable/Graph/Chain | ✅ | ✅ | ✅ |
| ReAct Agent (multi-turn) / 多轮 ReAct Agent | ✅ | ✅ | ✅ |
| MultiAgent patterns / MultiAgent 模式 | ✅ (4 modes) / ✅（4 种模式） | ✅ | ✅ |
| RAG with chunking / 带 chunking 的 RAG | ✅ (4 splitters) / ✅（4 种 splitter） | ✅ | ✅ |
| MCP (Model Context Protocol) / MCP（模型上下文协议） | ✅ | ✅ | ❌ |
| A2A (Agent-to-Agent) / A2A（Agent 到 Agent） | ✅ | ❌ | ❌ |
| WASM sandbox execution / WASM 沙箱执行 | ✅ | ❌ | ❌ |
| Cross-platform (Native + WASM) / 跨平台（Native + WASM） | ✅ | ❌ | ❌ |
| Pure Rust / 纯 Rust | ✅ | ❌ | ❌ |

**Synapse's unique strengths:**
*Synapse 的独特优势：*

- **A2A protocol** — First-class cross-process agent communication with persistent task storage
  *A2A 协议 —— 一等公民的跨进程 agent 通信，带持久化 task 存储*
- **WASM isolation** — Run agents in `wasi` sandboxes (PFC runtime)
  *WASM 隔离 —— agent 跑在 `wasi` 沙箱里（PFC runtime）*
- **MultiAgent 4-mode coverage** — Router, Orchestrator, Handoff, Hybrid
  *MultiAgent 四种模式全覆盖 —— Router、Orchestrator、Handoff、Hybrid*
- **MCP Server** — Build custom MCP servers with `SimpleMcpServer`
  *MCP Server —— 用 `SimpleMcpServer` 构建自定义 MCP server*
- **Type-safe Rust API** — No Python runtime overhead, async-native
  *类型安全的 Rust API —— 无 Python runtime 开销，原生异步*
- **292 tests** — Comprehensive unit + integration coverage, all mocked
  *292 个测试 —— 完整的单元 + 集成测试覆盖，全部 mock*

## Where to Go Next · 下一步

- **[Quick Start](/guide/quick-start)** — Build your first agent in 5 minutes
  *5 分钟构建你的第一个 agent*
- **[Architecture](/concepts/architecture)** — Understand the framework
  *理解框架架构*
- **[How-to Guides](/howto/simple-agent)** — Step-by-step recipes
  *手把手的实操教程*
- **[API Reference](/reference/agent-trait)** — Detailed API docs
  *详细 API 文档*

## Related Projects · 相关项目

- **Synapse** (this project) — Agent development toolkit
  *Synapse（本项目）—— agent 开发工具箱*
- **Neuron** — LLM serving runtime
  *Neuron —— LLM 推理服务 runtime*
- **PFC** — Multi-agent orchestrator + WASM sandbox runtime
  *PFC —— 多 agent 编排器 + WASM 沙箱 runtime*
