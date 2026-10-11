---
title: What is Synapse?
description: Rust-based intelligent agent development framework
layout: home

hero:
  name: Synapse
  text: Rust Agent Framework
  tagline: ReAct · RAG · MultiAgent · MCP · A2A · ACP · WASM
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
  - title: 🔌 MCP + A2A + ACP
    details: Model Context Protocol (stdio + HTTP + Server). Agent-to-Agent protocol. Agent Client Protocol client (drive opencode / claude-code / codex).
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

## Why Synapse?

| Capability | Synapse | LangChain | Eino |
|------------|---------|-----------|------|
| Runnable/Graph/Chain | ✅ | ✅ | ✅ |
| ReAct Agent (multi-turn) | ✅ | ✅ | ✅ |
| MultiAgent patterns | ✅ (4 modes) | ✅ | ✅ |
| RAG with chunking | ✅ (4 splitters) | ✅ | ✅ |
| MCP (Model Context Protocol) | ✅ | ✅ | ❌ |
| A2A (Agent-to-Agent) | ✅ | ❌ | ❌ |
| WASM sandbox execution | ✅ | ❌ | ❌ |
| Cross-platform (Native + WASM) | ✅ | ❌ | ❌ |
| Pure Rust | ✅ | ❌ | ❌ |

**Synapse's unique strengths:**

- **A2A protocol** — First-class cross-process agent communication with persistent task storage
- **ACP client** — Drive external coding agents (opencode, claude-code, codex) as normal Synapse streams
- **WASM isolation** — Run agents in `wasi` sandboxes (PFC runtime)
- **MultiAgent 4-mode coverage** — Router, Orchestrator, Handoff, Hybrid
- **MCP Server** — Build custom MCP servers with `SimpleMcpServer`
- **Type-safe Rust API** — No Python runtime overhead, async-native
- **292 tests** — Comprehensive unit + integration coverage, all mocked

## Where to Go Next

- **[Quick Start](/guide/quick-start)** — Build your first agent in 5 minutes
- **[Architecture](/concepts/architecture)** — Understand the framework
- **[How-to Guides](/howto/simple-agent)** — Step-by-step recipes
- **[API Reference](/reference/agent-trait)** — Detailed API docs

## Related Projects

- **Synapse** (this project) — Agent development toolkit
- **Neuron** — LLM serving runtime
- **PFC** — Multi-agent orchestrator + WASM sandbox runtime
