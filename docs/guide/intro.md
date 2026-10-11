---
title: Introduction
---

# Introduction · 简介

**Synapse** is a Rust-based intelligent agent development framework, comparable
to LangChain and Eino, providing the core building blocks for LLM-powered
applications: agents, tools, RAG, multi-agent coordination, MCP, A2A, ACP,
and WASM sandboxing.
*Synapse 是一个基于 Rust 的智能 agent 开发框架，与 LangChain、Eino 同类，为 LLM 应用提供核心构件：agent、tool、RAG、多 agent 协作、MCP、A2A、ACP 与 WASM 沙箱。*

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

- **A2A protocol** — First-class cross-process agent communication
  *A2A 协议 —— 一等公民的跨进程 agent 通信*
- **ACP client** — Drive external coding agents (opencode, claude-code, codex) as normal Synapse streams
  *ACP 客户端 —— 把外部编码 agent（opencode、claude-code、codex）接成普通的 Synapse 流*
- **WASM isolation** — Run agents in `wasi` sandboxes (PFC runtime)
  *WASM 隔离 —— agent 跑在 `wasi` 沙箱里（PFC runtime）*
- **MultiAgent 4-mode coverage** — Router, Orchestrator, Handoff, Hybrid
  *MultiAgent 四种模式全覆盖 —— Router、Orchestrator、Handoff、Hybrid*
- **Type-safe Rust API** — No Python runtime overhead, async-native
  *类型安全的 Rust API —— 无 Python runtime 开销，原生异步*
- **292 tests** — Comprehensive unit + integration coverage, all mocked
  *292 个测试 —— 完整的单元 + 集成测试覆盖，全部 mock*

## When to Use Synapse? · 什么时候用 Synapse？

Use Synapse when you need:
*当你需要以下能力时，选 Synapse：*

- LLM agents with tool use, ReAct, or graph-based orchestration
  *带 tool 调用、ReAct 或图式编排的 LLM agent*
- RAG pipelines (parsing → chunking → embedding → retrieval)
  *RAG pipeline（解析 → 分块 → embedding → 检索）*
- Multi-agent systems (routing, decomposition, handoff, hybrid)
  *多 agent 系统（路由、任务分解、handoff、混合）*
- MCP tool integration (stdio or HTTP)
  *MCP tool 集成（stdio 或 HTTP）*
- Cross-process agent communication (A2A)
  *跨进程 agent 通信（A2A）*
- Driving external coding agents (opencode, claude-code-acp, codex-acp) through the Agent Client Protocol
  *通过 Agent Client Protocol 驱动外部编码 agent（opencode、claude-code-acp、codex-acp）*
- Deployment to WASM sandboxes for isolation
  *部署到 WASM 沙箱以获得隔离*

## Project Status · 项目状态

- **Version:** 0.1.0
  *版本：0.1.0*
- **License:** Apache 2.0
  *许可证：Apache 2.0*
- **Tests:** 292 passing
  *测试：292 个全部通过*
- **Production-ready components:** Core agent/RAG/MCP/A2A/MultiAgent
  *可上生产的组件：核心 agent/RAG/MCP/A2A/MultiAgent*
- **Stable APIs:** Yes (semver respected)
  *API 稳定：是（遵守 semver）*

## Where to Go Next · 下一步

- [Quick Start](/guide/quick-start) — Build your first agent in 5 minutes
  *5 分钟构建你的第一个 agent*
- [Architecture](/concepts/architecture) — Understand the framework
  *理解框架架构*
- [How-to Guides](/howto/simple-agent) — Step-by-step recipes
  *手把手的实操教程*
- [API Reference](/reference/agent-trait) — Detailed API docs
  *详细 API 文档*

## Related Projects · 相关项目

- **Synapse** (this project) — Agent development toolkit
  *Synapse（本项目）—— agent 开发工具箱*
- **Neuron** — LLM serving runtime
  *Neuron —— LLM 推理服务 runtime*
- **PFC** — Multi-agent orchestrator + WASM sandbox runtime
  *PFC —— 多 agent 编排器 + WASM 沙箱 runtime*
