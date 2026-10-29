---
title: Lobe Module
---

# Lobe Module

The [`synapse::lobe`](../../src/lobe/mod.rs) module is a thin glue layer for building A2A-speaking agent **lobes** — self-contained services that own one or more skills and expose them as an A2A endpoint.

It is **not a framework** — it re-uses the existing `synapse::a2a::*`, `synapse::providers::*`, `synapse::transport::*` layers rather than wrapping or replacing them. The point is to compress the boilerplate of "wire a stateless skill executor to an A2A server with service registration and history summarization" into roughly 100 lines of `main.rs`.

Reference frameworks: LangChain (Python), BeeAI (Python), Eino (Go).

## Why a lobe module

A "lobe" in the Shadow architecture is one specialized service (temporal audio / occipital vision / parietal workflow / …). Each lobe needs to:

1. Expose one or more skills via A2A
2. Register itself with the encephalon registry
3. Stay stateless — conversation history flows in via `SkillContext`, out via `SkillOutput.updated_history_summary`, caller persists
4. Bundle LLM + TTS + ASR + Emotion providers as one `ProviderSet`
5. Optionally summarize long history via `HistorySummarizer`

`synapse::lobe` packages all five so a lobe author writes the skill — not the A2A plumbing.

## Anatomy

```
synapse::lobe/
├── mod.rs                     # module root + re-exports
├── stateless_skill.rs         # StatelessSkill + SkillContext + SkillOutput + JsonCtx + Router
├── provider_set.rs            # ProviderSet (LLM + TTS + ASR bundle)
├── provider_traits.rs         # EmotionProvider / EmotionInput / EmotionOutput
├── env_config.rs              # LobeEnv trait (env-var config)
├── history.rs                 # HistorySummarizer (LLM-driven summary)
├── metrics.rs                 # lobe-level metrics (calls, tokens, errors)
├── service_registration.rs    # register_with_registry (POST AgentCard)
├── encephalon_registration.rs # register_with_encephalon (one-call registration)
├── a2a_server.rs              # build_a2a_router (axum handler)
├── lobe_app.rs                # LobeApp 10-line convenience builder
├── config_manager.rs          # ConfigManager + GracefulReload + LobeConfig
├── wasm_runtime.rs            # WasmLobeRuntime (call WASM skills)
└── media_pipeline.rs          # AsrResult / MediaProcessor / VadState / VideoFrame
```

## Quick start

```rust,no_run
use std::sync::Arc;
use synapse::lobe::{LobeApp, StatelessSkill, SkillContext, SkillOutput};
use serde_json::json;

struct EchoSkill;

#[async_trait::async_trait]
impl StatelessSkill for EchoSkill {
    type Input = String;
    type Output = String;
    async fn run(&self, input: Self::Input, _ctx: &SkillContext) -> SkillOutput<Self::Output> {
        SkillOutput::ok(input)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    LobeApp::new("echo", "Echo skill")
        .skill(Arc::new(EchoSkill))
        .serve("0.0.0.0:9101").await?;
    Ok(())
}
```

You automatically get:

- **A2A server** (axum) listening on the requested port
- **AgentCard auto-published** to the encephalon registry at boot
- **`StatelessSkillRouter`** dispatch on `skill_id`
- **Graceful reload** of `LobeConfig` on SIGHUP
- **lobe::metrics** counters wired in

## Core traits

### `StatelessSkill`

```rust
#[async_trait]
pub trait StatelessSkill: Send + Sync {
    type Input: DeserializeOwned;
    type Output: Serialize;
    fn skill_id(&self) -> &'static str;
    async fn run(&self, input: Self::Input, ctx: &SkillContext) -> SkillOutput<Self::Output>;
}
```

A skill is a single async function that takes typed input + context and returns typed output. **No state lives in the skill itself between calls** — conversation history flows in via `SkillContext.history_summary` and out via `SkillOutput.updated_history_summary`.

### `SkillContext`

```rust
pub trait SkillContext: Send + Sync {
    fn user_id(&self) -> &str;
    fn history_summary(&self) -> Option<&str> { None }
    fn system_prompt(&self) -> Option<&str> { None }
    fn extra(&self) -> Option<&serde_json::Value> { None }
}
```

A free-form JSON context object that travels alongside every skill call. The `JsonCtx` struct gives you a default `user_id` + `history_summary` + `extra` for free.

### `SkillOutput`

```rust
pub struct SkillOutput {
    pub updated_history_summary: Option<String>,
    pub payload: serde_json::Value,
    pub audio_url: Option<String>,
}
```

Carries the skill's response payload + optional audio output + optional new history summary (caller persists).

### `EmotionProvider`

```rust
#[async_trait]
pub trait EmotionProvider: Send + Sync {
    async fn detect(&self, input: EmotionInput) -> EmotionOutput;
}
```

Lobes that need emotion-aware responses (e.g. companion lobes) implement this; the rest can ignore it. Returns valence / arousal / dominance.

### `LobeEnv`

```rust
pub trait LobeEnv {
    fn bind_addr(&self) -> SocketAddr;
    fn registry_url(&self) -> &str;
    fn provider_set(&self) -> &ProviderSet;
    // ... override per-lobe
}
```

Env-driven config trait: each lobe loads its own env vars (`LOBE_BIND`, `LOBE_REGISTRY_URL`, `OPENAI_API_KEY`, …) and produces a `LobeEnv` implementation.

## Helpers

### `ProviderSet`

```rust
pub struct ProviderSet {
    pub llm: Arc<dyn LLMProvider>,
    pub tts: Option<Arc<dyn TtsProvider>>,
    pub asr: Option<Arc<dyn AsrProvider>>,
    pub emotion: Option<Arc<dyn EmotionProvider>>,
}
```

One bundle that holds every provider the lobe needs. Constructed once in `main.rs`, cloned cheaply into every skill.

### `HistorySummarizer`

```rust
pub struct HistorySummarizer { /* … */ }

impl HistorySummarizer {
    pub fn new(llm: Arc<dyn LLMProvider>, max_tokens: usize) -> Self;
    pub async fn summarize(&self, history: &str) -> Result<String, String>;
}
```

LLM-driven summarizer used when `SkillContext.history_summary` is too long to fit the model context. Lobes that don't need long history can skip this entirely.

### `StatelessSkillRouter`

```rust
let router = StatelessSkillRouter::new()
    .add("echo", Arc::new(EchoSkill))
    .add("summarize", Arc::new(SummarizeSkill));
```

Maps `skill_id` → `Arc<dyn StatelessSkill>` for A2A dispatch. Built into `LobeApp.skill(...)` automatically.

## A2A registration

`LobeApp.serve(...)` boots the axum A2A server and registers the lobe with the encephalon registry. If you want finer control:

```rust
use synapse::lobe::{build_a2a_router, register_with_encephalon, AgentCard};

let card = AgentCard::new("my-lobe", "My specialized lobe", "http://0.0.0.0:9101", "1.0");
let router = build_a2a_router(card.clone(), skill_router);
register_with_encephalon(&card, "http://encephalon:60080").await?;

axum::serve(tcp_listener, router).await?;
```

## `LobeApp` builder

| Method | Purpose |
|------|------|
| `LobeApp::new(name, description)` | Start a new app |
| `.bind(addr)` | Override bind address |
| `.skill(Arc<dyn StatelessSkill>)` | Register a skill |
| `.provider_set(ps)` | Inject a custom ProviderSet |
| `.env(LobeEnv impl)` | Load env-driven config |
| `.serve(addr)` | Boot axum + register with encephalon + run forever |

## When to use `synapse::lobe`

| Scenario | Use lobe? |
|------|:---:|
| New specialized agent service (audio / vision / workflow) | ✅ |
| One-shot script or library consumer | ❌ use `synapse::a2a::*` directly |
| Embedding agent logic inside another Rust crate | ❌ use `synapse::adk::*` |
| A WASM sandboxed skill | ✅ via `WasmLobeRuntime` |
| A pure-Rust skill that needs emotion | ✅ |

## See also

- [`src/lobe/mod.rs`](../../src/lobe/mod.rs) — module root + rustdoc
- [`src/lobe/lobe_app.rs`](../../src/lobe/lobe_app.rs) — `LobeApp` convenience builder
- [`src/lobe/stateless_skill.rs`](../../src/lobe/stateless_skill.rs) — `StatelessSkill` trait
- [`src/lobe/provider_set.rs`](../../src/lobe/provider_set.rs) — `ProviderSet`
- [`src/lobe/history.rs`](../../src/lobe/history.rs) — `HistorySummarizer`
- [`src/lobe/a2a_server.rs`](../../src/lobe/a2a_server.rs) — `build_a2a_router`
- [`docs/concepts/a2a.md`](a2a.md) — A2A protocol reference
- [`docs/concepts/multiagent.md`](multiagent.md) — MultiAgent supervisors (consumers of lobes)
