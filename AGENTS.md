# Synapse — 神经突触层 (Shared Foundation Library)

**Updated**: 2026-07-19 (zero-warnings pass + A2A metadata wire-forwarding + WASM-A2A bridge sync path)
**Status**: ✅ v0.4 lobe module complete + WASM-A2A sync bridge + A2A metadata round-trip via `__encephalon_meta_v1__`

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
├── providers/              # LLMProvider trait + implementations + audio::{Asr,Tts}Provider (added 2026-08-22)
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
- `register_with_encephalon()` — POST AgentCard to encephalon
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
- reqwest 0.13 — HTTP client (with multipart feature for audio providers)
- ziti-sdk 0.3 — **optional**, behind `feature = "ziti"` (zero-trust A2A over OpenZiti)

## `transport::ziti` module (added 2026-09-09)

Zero-trust A2A over OpenZiti. Gated by `feature = "ziti"`
because there is **no official OpenZiti Rust SDK** — the only
binding on crates.io is the third-party `jaschadub/ziti-rust`
v0.3.x (MIT, community-maintained). Default callers don't pull
`ziti-sdk`; opt in when you need dark-service mode.

Public surface:
- `ZitiGateway::new(identity_path)` — load an OpenZiti identity
  JWT, get a context.
- `gateway.dial(service_name)` / `gateway.listen(service_name)`
  — peer connection + dark-service listener.
- `ZitiListener::accept()` — pull one incoming `ZitiStream`.
- `serve(router, identity_path, service_name)` — one-call convenience
  that does identity load + dark-service register + accept loop +
  `serve_ziti_stream` per connection. Replaces the 4-line dance
  callers used to write.
- `serve_ziti_stream(stream, router)` — serve one axum `Router`
  on one accepted `ZitiStream` (in-process glue: hyper over Ziti).
- `ZitiHttpClient` / `ConnectionPool` / `ZitiConfig` /
  `ConfigWatcher` — outbound HTTP + reusable streams + JSON config.

### SDK caveat (read before depending on it)

OpenZiti ships official SDKs for Go / C / Swift / Java / Python /
C# but **not Rust**. The `ziti-sdk` crate we depend on is the
only Rust binding on crates.io and is community-maintained, not
by the OpenZiti project. **Vendor strategy recommended for
production** — fork `jaschadub/ziti-rust` into the org namespace
and pin via `[patch.crates-io]` so a stuck upstream doesn't
take encephalon with it.

### Migration posture

The module is insulated from callers by the `ZitiGateway` /
`ZitiListener` / `ZitiStream` API. Swapping to a different SDK
(including an eventual official Rust binding) is a local change
to `gateway.rs` + `listener.rs`; downstream code is unaffected.

## Audio Providers (added 2026-08-22)

```rust
use synapse::providers::audio::{AsrProvider, TtsProvider};

let asr = AsrProvider::for_endpoint("http://localhost:60080".into(), "whisper-1".into());
let text = asr.transcribe(audio_bytes, "wav", Some("zh")).await?;

let tts = TtsProvider::for_endpoint("http://localhost:60080".into(), "tts-1".into(), "alloy".into());
let mp3 = tts.synthesize("你好", "mp3").await?;
```

替代各 Agent 中直接 POST `/v1/audio/{transcriptions,speech}` 的样板代码。

## LLM Provider convenience (added 2026-08-22)

- `OpenAIProvider::for_endpoint(base_url, model)` — 内部 OpenAI 兼容 endpoint 构造（无 API key）
- `Message::user(text)` / `Message::assistant(text)` / `Message::system(text)` — 工厂方法
- `Message::text()` — 提取纯文本响应
- `AgentCard::placeholder(name)` — 占位 AgentCard（替代 `new(name, "", "", "0.0.0")`）
- `ReqwestHttpClient::with_timeout(Duration)` — 带超时的 reqwest HTTP 客户端

## Commits

| Hash | Description |
|------|-------------|
| 1728752 | chore: eliminate all remaining warnings (89 → 0) across the workspace |
| 135f1fd | feat: forward encephalon's packed `metadata['name']` through `into_a2a_message` via `__encephalon_meta_v1__` prefix (decode via `decode_metadata_from_name`) — closes the v0.1 metadata propagation gap |
| 9b19949 | docs: allow `ambiguous_glob_reexports` in lib.rs (cosmetic lint exemption, documented) |
| WASM-A2A bridge | `call_entry` reads N bytes from linear memory at offset `ptr` (was returning placeholder); combined with encephalon's `invoke` no longer discarding `_result`, real WASM agents with `handle(json) -> json` reach encephalon end-to-end |
| 754c5a8 | feat: add synapse::lobe module — v0.4 stateless skill lifecycle glue |
| 207bef6 | docs: migrate project-level + family docs to /home/ibloc/Projects/docs/ |
| c96f4f6 | docs: phase 4.5 status + prefrontal URL convention in shadow contract |
