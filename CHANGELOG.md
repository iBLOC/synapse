# Changelog

All notable changes to Synapse are documented here.

## [Unreleased]

### Added
- **ACP client** (feature `acp`) — drive external coding agents (OpenCode, Claude Code via `claude-agent-acp`, Codex via `codex-acp`, …) over stdio via the official `agent-client-protocol` SDK; prompts stream back as `AgentStreamEvent`s
- **MultiAgent HybridSupervisor** — LLM-driven routing + sub-agent handoff
- **MultiAgent HandoffSupervisor** — sub-agents decide handoff via `event.action.transfer_to`
- **MultiAgent OrchestratorAgent** — task decomposition with parallel dispatch
- **MultiAgent RouterSupervisor** — LLM routes to sub-agents via tool calls
- **DocumentTransformer trait + 3 chunkers** — Character / RecursiveCharacter / MarkdownHeader
- **MCP client (stdio)** — Model Context Protocol integration
- **A2A Adapter** — Agent-to-Agent protocol
- **WasiHttpClient** — wasip2 HTTP for WASM sandbox execution
- **OpenAIClient + AnthropicClient** — multi-provider LLM support
- **InMemory RAG** — VectorStore / Indexer / Retriever with cosine similarity
- **OpenAI Embedding** — text-embedding-3-small support
- **GraphReActAgent** — graph-based multi-turn ReAct agent
- **ToolMiddleware** — pre/post hooks for tool execution
- **DocumentParser** — Text + JSON parsers
- **Runner** — Agent executor with state management

### Refactored
- **Unified Document type** — `document::Document` is canonical, `retriever::Document` is re-export
- **Unified Document::text()** — convenient constructor with default mime_type

### Fixed
- **ACP client dropped agent message chunks** — `session/update` notifications were parsed as a bare `SessionUpdate` instead of the `SessionNotification` envelope (`{sessionId, update}`), so `AgentMessageChunk` text never reached consumers; now tries the envelope first and logs unparsable notifications at debug level instead of dropping silently

### Tests
- 301 tests passing with `--features acp`, 292 default (no external dependencies, all mocked)

---

## Specs (Design Documents)

- `docs/superpowers/specs/2026-03-26-phase3-graph-react-agent-design.md` — GraphReActAgent
- `docs/superpowers/specs/2026-06-26-wasm-a2a-design.md` — WASM/A2A transport
- `docs/superpowers/specs/2026-06-28-multiagent-router-design.md` — Router/Supervisor
- `docs/superpowers/specs/2026-06-28-multiagent-orchestrator-design.md` — Host/Orchestrator
- `docs/superpowers/specs/2026-06-28-multiagent-handoff-design.md` — Conversational Handoff
- `docs/superpowers/specs/2026-06-28-multiagent-handoff-design.md` — Hybrid
- `docs/superpowers/specs/2026-06-28-document-transformer-design.md` — Chunking
- `docs/superpowers/specs/2026-06-28-unify-document-type.md` — Document refactor
