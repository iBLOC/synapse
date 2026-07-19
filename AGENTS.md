# Synapse — 神经突触层 (Shared Foundation Library)

**Updated**: 2026-07-19 (zero-warnings pass + A2A metadata wire-forwarding + WASM-A2A bridge sync path)
**Status**: ✅ v0.4 lobe module complete + WASM-A2A sync bridge + A2A metadata round-trip via `__cortex_meta_v1__`

## Role

Shared foundation library for the Shadow platform. Provides A2A protocol, WASM runtime, LLM providers, and the new `synapse::lobe` module for stateless skill lifecycle.

## Architecture

```
synapse/src/
├── a2a/                    # Agent-to-Agent protocol (card, client, server)
├── adk/                    # Agent Development Kit (OrchestratorAgent)
├── agent/                  # Agent trait + orchestrator
├── callbacks/              # Event callbacks
├── channels/               # Channel plugins (dock, router)
├── coding_tools/           # Code-shaped tool helpers (added 2026-07+)
├── components/             # Model, RAG, embedding, indexer
├── compose/                # Graph composition (chain, workflow)
├── llm/                    # LLM implementations (OpenAI, Anthropic)
├── lobe/                   # Stateless skill lifecycle glue (v0.4)
├── mcp/                    # Model Context Protocol
├── memory/                 # Memory helpers (added 2026-07+)
├── protocol/               # HybridRAG, Rerank, Retriever
├── providers/              # LLMProvider trait + implementations
├── scheduler/              # Task scheduling
├── schema/                 # Message, Content, RoleType
├── session/                # Session management
├── skills/                 # Skill definitions
├── stream/                 # Stream reader/writer
├── structured/             # Structured output helpers (added 2026-07+)
├── task/                   # Task management
├── thought/                # Thought chains
├── trace_context.rs        # Distributed tracing context (added 2026-07+)
├── transport/              # HTTP client
├── utils/                  # Utilities
└── wasm/                   # WASM runtime (package, manager, registry) — call_entry reads N bytes from linear memory at offset `ptr` (closes WASM-A2A sync bridge)
```

## Build

```bash
cd synapse
cargo build
```

## Test

```bash
cd synapse
cargo test --lib lobe  # lobe module tests
```

## lobe Module (v0.4)

The `synapse::lobe` module provides lifecycle glue for stateless skill executors:

### Traits
- `StatelessSkill` — Every lobe skill implements this
- `SkillContext` — Context passed to skill (user_id, history_summary, extra)
- `LobeEnv` — Env-driven config trait
- `EmotionProvider` — Emotion detection trait

### Helpers
- `HistorySummarizer` — LLM summarization on overflow
- `ProviderSet` — Bundles TTS/ASR/LLM/Emotion providers
- `register_with_cortex()` — POST AgentCard to cortex
- `build_a2a_router()` — axum router for A2A endpoints
- `LobeApp` — Convenience builder (register + serve in 10 lines)

### Re-exports
- `a2a::card::*` — AgentCard, Authentication, Capabilities, Provider, SkillReference
- `a2a::protocol::*` — Message, MessagePart, MessageRole, Task, TaskState
- `providers::LLMProvider` — LLM provider trait

## Dependencies

- tokio 1 — Async runtime
- serde 1 — Serialization
- async-trait 0.1 — Async traits
- axum 0.8 — HTTP server (for a2a_server)
- reqwest 0.12 — HTTP client

## Commits

| Hash | Description |
|------|-------------|
| 1728752 | chore: eliminate all remaining warnings (89 → 0) across the workspace |
| 135f1fd | feat: forward cortex's packed `metadata['name']` through `into_a2a_message` via `__cortex_meta_v1__` prefix (decode via `decode_metadata_from_name`) — closes the v0.1 metadata propagation gap |
| 9b19949 | docs: allow `ambiguous_glob_reexports` in lib.rs (cosmetic lint exemption, documented) |
| WASM-A2A bridge | `call_entry` reads N bytes from linear memory at offset `ptr` (was returning placeholder); combined with cortex's `invoke` no longer discarding `_result`, real WASM agents with `handle(json) -> json` reach cortex end-to-end |
| 754c5a8 | feat: add synapse::lobe module — v0.4 stateless skill lifecycle glue |
| 207bef6 | docs: migrate project-level + family docs to /home/ibloc/Projects/docs/ |
| c96f4f6 | docs: phase 4.5 status + prefrontal URL convention in shadow contract |
