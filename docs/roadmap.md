---
title: Roadmap
---

# Roadmap

Planned future directions for Synapse.

## Current (0.1.x)

- ✅ Core Agent trait
- ✅ ReAct agent (ChatModelAgent, GraphReActAgent)
- ✅ Tool framework (BaseTool, InvokableTool, EnhancedInvokableTool)
- ✅ MultiAgent (Router, Orchestrator, Handoff, Hybrid)
- ✅ RAG (chunking, embedding, vector store, retriever)
- ✅ MCP client (stdio + HTTP)
- ✅ A2A protocol
- ✅ Cross-platform (native + WASM)
- ✅ OpenAI + Anthropic LLM support
- ✅ Document unification + JSON serialization
- ✅ Apache 2.0 licensed
- ✅ 205 tests

## Short Term (0.2.x)

- [ ] More vector store backends (LanceDB, Qdrant, pgvector)
- [ ] Streaming response support for all chat models
- [ ] Full resume/checkpoint for long-running agents
- [ ] Built-in A2A server (axum-based)
- [ ] More document parsers (PDF, DOCX, etc.)
- [ ] HTML-aware chunker
- [ ] Code-aware chunker (function/class boundaries)

## Medium Term (0.3.x+)

- [ ] Agent observability (OpenTelemetry integration)
- [ ] Prompt template DSL
- [ ] Agent memory management (short-term + long-term)
- [ ] Multi-modal support (vision, audio)
- [ ] Distributed agent execution
- [ ] PFC runtime integration (deploy to PFC)
- [ ] Neuron LLM runtime integration

## Long Term

- [ ] Visual agent builder (no-code)
- [ ] Agent marketplace / registry
- [ ] Performance benchmarks
- [ ] Production deployment guides

## Contributing

Want to help shape the roadmap? Open an issue on GitHub or check
out [CONTRIBUTING.md](https://github.com/ibloc/synapse/blob/master/CONTRIBUTING.md).
