---
title: Roadmap
---

# Roadmap · 路线图

Planned future directions for Synapse.
*Synapse 的未来规划方向。*

## Current (0.1.x) · 当前（0.1.x）

- ✅ Core Agent trait
  *✅ 核心 Agent trait*
- ✅ ReAct agent (ChatModelAgent, GraphReActAgent)
  *✅ ReAct agent（ChatModelAgent、GraphReActAgent）*
- ✅ Tool framework (BaseTool, InvokableTool, EnhancedInvokableTool)
  *✅ Tool 框架（BaseTool、InvokableTool、EnhancedInvokableTool）*
- ✅ MultiAgent (Router, Orchestrator, Handoff, Hybrid)
  *✅ MultiAgent（Router、Orchestrator、Handoff、Hybrid）*
- ✅ RAG (chunking, embedding, vector store, retriever)
  *✅ RAG（分块、embedding、向量库、检索器）*
- ✅ MCP client (stdio + HTTP)
  *✅ MCP client（stdio + HTTP）*
- ✅ A2A protocol
  *✅ A2A 协议*
- ✅ Cross-platform (native + WASM)
  *✅ 跨平台（native + WASM）*
- ✅ OpenAI + Anthropic LLM support
  *✅ OpenAI + Anthropic LLM 支持*
- ✅ Document unification + JSON serialization
  *✅ Document 统一 + JSON 序列化*
- ✅ Apache 2.0 licensed
  *✅ Apache 2.0 许可*
- ✅ 205 tests
  *✅ 205 个测试*

## Short Term (0.2.x) · 短期（0.2.x）

- [ ] More vector store backends (LanceDB, Qdrant, pgvector)
  *更多向量库后端（LanceDB、Qdrant、pgvector）*
- [ ] Streaming response support for all chat models
  *所有 chat model 支持流式响应*
- [ ] Full resume/checkpoint for long-running agents
  *长时运行 agent 的完整 resume/checkpoint*
- [ ] Built-in A2A server (axum-based)
  *内置 A2A server（基于 axum）*
- [ ] More document parsers (PDF, DOCX, etc.)
  *更多文档解析器（PDF、DOCX 等）*
- [ ] HTML-aware chunker
  *感知 HTML 结构的 chunker*
- [ ] Code-aware chunker (function/class boundaries)
  *感知代码结构的 chunker（按函数/类边界）*

## Medium Term (0.3.x+) · 中期（0.3.x+）

- [ ] Agent observability (OpenTelemetry integration)
  *Agent 可观测性（集成 OpenTelemetry）*
- [ ] Prompt template DSL
  *Prompt 模板 DSL*
- [ ] Agent memory management (short-term + long-term)
  *Agent 记忆管理（短期 + 长期）*
- [ ] Multi-modal support (vision, audio)
  *多模态支持（视觉、音频）*
- [ ] Distributed agent execution
  *分布式 agent 执行*
- [ ] PFC runtime integration (deploy to PFC)
  *PFC runtime 集成（部署到 PFC）*
- [ ] Neuron LLM runtime integration
  *Neuron LLM runtime 集成*

## Long Term · 长期

- [ ] Visual agent builder (no-code)
  *可视化 agent 构建器（no-code）*
- [ ] Agent marketplace / registry
  *Agent 市场 / 注册中心*
- [ ] Performance benchmarks
  *性能基准测试*
- [ ] Production deployment guides
  *生产部署指南*

## Contributing · 参与贡献

Want to help shape the roadmap? Open an issue on GitHub or check
out [CONTRIBUTING.md](https://github.com/ibloc/synapse/blob/master/CONTRIBUTING.md).
*想参与路线图的规划？在 GitHub 上开 issue，或查阅 [CONTRIBUTING.md](https://github.com/ibloc/synapse/blob/master/CONTRIBUTING.md)。*
