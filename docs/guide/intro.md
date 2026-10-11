---
title: Introduction
---

# Introduction

**Synapse** is a Rust-based intelligent agent development framework, comparable
to LangChain and Eino, providing the core building blocks for LLM-powered
applications: agents, tools, RAG, multi-agent coordination, MCP, A2A, ACP,
and WASM sandboxing.

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

- **A2A protocol** — First-class cross-process agent communication
- **ACP client** — Drive external coding agents (opencode, claude-code, codex) as normal Synapse streams
- **WASM isolation** — Run agents in `wasi` sandboxes (Telencephalon runtime)
- **MultiAgent 4-mode coverage** — Router, Orchestrator, Handoff, Hybrid
- **Type-safe Rust API** — No Python runtime overhead, async-native
- **292 tests** — Comprehensive unit + integration coverage, all mocked

## When to Use Synapse?

Use Synapse when you need:

- LLM agents with tool use, ReAct, or graph-based orchestration
- RAG pipelines (parsing → chunking → embedding → retrieval)
- Multi-agent systems (routing, decomposition, handoff, hybrid)
- MCP tool integration (stdio or HTTP)
- Cross-process agent communication (A2A)
- Driving external coding agents (opencode, claude-code-acp, codex-acp) through the Agent Client Protocol
- Deployment to WASM sandboxes for isolation

## Project Status

- **Version:** 0.1.0
- **License:** Apache 2.0
- **Tests:** 292 passing
- **Production-ready components:** Core agent/RAG/MCP/A2A/MultiAgent
- **Stable APIs:** Yes (semver respected)

## Where to Go Next

- [Quick Start](/guide/quick-start) — Build your first agent in 5 minutes
- [Architecture](/concepts/architecture) — Understand the framework
- [How-to Guides](/howto/simple-agent) — Step-by-step recipes
- [API Reference](/reference/agent-trait) — Detailed API docs

## Related Projects

- **Synapse** (this project) — Agent development toolkit
- **Neuron** — LLM serving runtime
- **Telencephalon** (端脑) — Multi-agent execution runtime: agent registry, A2A gateway, WASM agent host
