---
title: Lobe Module
---

# Lobe Module · Lobe 模块（v0.4）

The [`synapse::lobe`](../../src/lobe/mod.rs) module is a thin glue layer for building A2A-speaking agent **lobes** — self-contained services that own one or more skills and expose them as an A2A endpoint.

[`synapse::lobe`](../../src/lobe/mod.rs) 是一个薄薄的 glue 层，用来构建讲 A2A 协议的 agent **lobe** —— 自包含的服务，承载一个或多个 skill，并通过 A2A endpoint 暴露给外部。

It is **not a framework** — it re-uses the existing `synapse::a2a::*`, `synapse::providers::*`, `synapse::transport::*` layers rather than wrapping or replacing them. The point is to compress the boilerplate of "wire a stateless skill executor to an A2A server with service registration and history summarization" into roughly 100 lines of `main.rs`.
*它**不是一个框架** —— 它复用现成的 `synapse::a2a::*`、`synapse::providers::*`、`synapse::transport::*` 层而不是包一层。目标是把"把 stateless skill executor 接到 A2A server、配上服务注册与历史摘要"的样板压到 100 行 `main.rs`。*

Reference frameworks: LangChain (Python), BeeAI (Python), Eino (Go).
*参考框架：LangChain（Python）、BeeAI（Python）、Eino（Go）。*

## Why a lobe module · 为什么需要 lobe 模块

A "lobe" in the Shadow brain architecture is one specialized service (temporal audio / occipital vision / parietal workflow / …). Each lobe needs to:
*Shadow 大脑架构里一个 lobe = 一个专门服务（颞叶音频 / 枕叶视觉 / 顶叶工作流 / …）。每个 lobe 都要：*

1. Expose one or more skills via A2A / 通过 A2A 暴露一个或多个 skill
2. Register itself with the brain registry (encephalon) / 在脑 registry（encephalon）注册自己
3. Stay stateless — conversation history flows in via `SkillContext`, out via `SkillOutput.updated_history_summary`, caller persists / 保持无状态 —— 对话历史从 `SkillContext` 流入、从 `SkillOutput.updated_history_summary` 流出，由调用方持久化
4. Bundle LLM + TTS + ASR + Emotion providers as one `ProviderSet` / 把 LLM + TTS + ASR + Emotion provider 打包为一个 `ProviderSet`
5. Optionally summarize long history via `HistorySummarizer` / 长历史可选走 `HistorySummarizer` 摘要

`synapse::lobe` packages all five so a lobe author writes the skill — not the A2A plumbing.
*`synapse::lobe` 把这五件事打包好，让 lobe 作者只写 skill，不用碰 A2A plumbing。*

## Anatomy · 解剖

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

## Quick start · 快速开始

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
*自动获得：*

- **A2A server** (axum) listening on the requested port / A2A server（axum）监听所请求端口
- **AgentCard auto-published** to the brain registry at boot / AgentCard 在启动时自动发到 brain registry
- **`StatelessSkillRouter`** dispatch on `skill_id` / `StatelessSkillRouter` 按 `skill_id` dispatch
- **Graceful reload** of `LobeConfig` on SIGHUP / SIGHUP 时优雅 reload `LobeConfig`
- **lobe::metrics** counters wired in / 接好 `lobe::metrics` 计数器

## Core traits · 核心 traits

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
*Skill 是一个 async 函数，输入是 typed `Input` + `SkillContext`，输出是 typed `Output`。**调用之间 skill 内部不持有状态** —— 对话历史通过 `SkillContext.history_summary` 流入、通过 `SkillOutput.updated_history_summary` 流出。*

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
*伴随每次 skill 调用的自由 JSON context。`JsonCtx` 结构开箱即用地提供 `user_id` + `history_summary` + `extra`。*

### `SkillOutput`

```rust
pub struct SkillOutput {
    pub updated_history_summary: Option<String>,
    pub payload: serde_json::Value,
    pub audio_url: Option<String>,
}
```

Carries the skill's response payload + optional audio output + optional new history summary (caller persists).
*承载 skill 响应 payload + 可选音频输出 + 可选新历史摘要（由调用方持久化）。*

### `EmotionProvider`

```rust
#[async_trait]
pub trait EmotionProvider: Send + Sync {
    async fn detect(&self, input: EmotionInput) -> EmotionOutput;
}
```

Lobes that need emotion-aware responses (e.g. companion lobes) implement this; the rest can ignore it. Returns valence / arousal / dominance.
*需要情绪感知响应的 lobe（比如陪伴 lobe）实现这个 trait；其他 lobe 可以忽略。返回 valence / arousal / dominance。*

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
*环境变量驱动的配置 trait：每个 lobe 加载自己的环境变量（`LOBE_BIND`、`LOBE_REGISTRY_URL`、`OPENAI_API_KEY` …）并产出 `LobeEnv` 实现。*

## Helpers · 助手

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
*一个 lobe 所需全部 provider 的合集。在 `main.rs` 构造一次，便宜地克隆到每个 skill 里。*

### `HistorySummarizer`

```rust
pub struct HistorySummarizer { /* … */ }

impl HistorySummarizer {
    pub fn new(llm: Arc<dyn LLMProvider>, max_tokens: usize) -> Self;
    pub async fn summarize(&self, history: &str) -> Result<String, String>;
}
```

LLM-driven summarizer used when `SkillContext.history_summary` is too long to fit the model context. Lobes that don't need long history can skip this entirely.
*当 `SkillContext.history_summary` 太长装不进模型 context 时使用的 LLM 摘要器。不需要长历史的 lobe 可以完全跳过。*

### `StatelessSkillRouter`

```rust
let router = StatelessSkillRouter::new()
    .add("echo", Arc::new(EchoSkill))
    .add("summarize", Arc::new(SummarizeSkill));
```

Maps `skill_id` → `Arc<dyn StatelessSkill>` for A2A dispatch. Built into `LobeApp.skill(...)` automatically.
*把 `skill_id` 映射到 `Arc<dyn StatelessSkill>` 用于 A2A dispatch。`LobeApp.skill(...)` 自动接好。*

## A2A registration · A2A 注册

`LobeApp.serve(...)` boots the axum A2A server and registers the lobe with the brain registry. If you want finer control:
*`LobeApp.serve(...)` 启动 axum A2A server 并把 lobe 注册到 brain registry。需要更细粒度时：*

```rust
use synapse::lobe::{build_a2a_router, register_with_encephalon, AgentCard};

let card = AgentCard::new("my-lobe", "My specialized lobe", "http://0.0.0.0:9101", "1.0");
let router = build_a2a_router(card.clone(), skill_router);
register_with_encephalon(&card, "http://encephalon:60080").await?;

axum::serve(tcp_listener, router).await?;
```

## `LobeApp` builder · `LobeApp` 构造器

| Method / 方法 | Purpose / 用途 |
|------|------|
| `LobeApp::new(name, description)` | Start a new app / 新建应用 |
| `.bind(addr)` | Override bind address / 覆盖绑定地址 |
| `.skill(Arc<dyn StatelessSkill>)` | Register a skill / 注册 skill |
| `.provider_set(ps)` | Inject a custom ProviderSet / 注入自定义 ProviderSet |
| `.env(LobeEnv impl)` | Load env-driven config / 加载环境变量配置 |
| `.serve(addr)` | Boot axum + register with encephalon + run forever / 启 axum + 注册 encephalon + 永久运行 |

## When to use `synapse::lobe` · 何时用 `synapse::lobe`

| Scenario / 场景 | Use lobe? / 用 lobe? |
|------|:---:|
| New specialized agent service (audio / vision / workflow) / 新的专门 agent 服务（音频 / 视觉 / 工作流） | ✅ |
| One-shot script or library consumer / 一次性脚本或库调用方 | ❌ use `synapse::a2a::*` directly / 直接用 `synapse::a2a::*` |
| Embedding agent logic inside another Rust crate / 把 agent 逻辑嵌入另一个 Rust crate | ❌ use `synapse::adk::*` / 直接用 `synapse::adk::*` |
| A WASM sandboxed skill / WASM 沙箱 skill | ✅ via `WasmLobeRuntime` |
| A pure-Rust skill that needs emotion / 需要情绪的纯 Rust skill | ✅ |

## See also · 相关阅读

- [`src/lobe/mod.rs`](../../src/lobe/mod.rs) — module root + rustdoc
- [`src/lobe/lobe_app.rs`](../../src/lobe/lobe_app.rs) — `LobeApp` convenience builder
- [`src/lobe/stateless_skill.rs`](../../src/lobe/stateless_skill.rs) — `StatelessSkill` trait
- [`src/lobe/provider_set.rs`](../../src/lobe/provider_set.rs) — `ProviderSet`
- [`src/lobe/history.rs`](../../src/lobe/history.rs) — `HistorySummarizer`
- [`src/lobe/a2a_server.rs`](../../src/lobe/a2a_server.rs) — `build_a2a_router`
- [`docs/concepts/a2a.md`](a2a.md) — A2A protocol reference
- [`docs/concepts/multiagent.md`](multiagent.md) — MultiAgent supervisors (consumers of lobes)
