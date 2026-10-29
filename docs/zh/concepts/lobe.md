---
title: Lobe Module
---

# Lobe 模块（v0.4）

[`synapse::lobe`](../../src/lobe/mod.rs) 是一个薄薄的 glue 层，用来构建讲 A2A 协议的 agent **lobe** —— 自包含的服务，承载一个或多个 skill，并通过 A2A endpoint 暴露给外部。

它**不是一个框架** —— 它复用现成的 `synapse::a2a::*`、`synapse::providers::*`、`synapse::transport::*` 层而不是包一层。目标是把"把 stateless skill executor 接到 A2A server、配上服务注册与历史摘要"的样板压到 100 行 `main.rs`。

参考框架：LangChain（Python）、BeeAI（Python）、Eino（Go）。

## 为什么需要 lobe 模块

Shadow 大脑架构里一个 lobe = 一个专门服务（颞叶音频 / 枕叶视觉 / 顶叶工作流 / …）。每个 lobe 都要：

1. 通过 A2A 暴露一个或多个 skill
2. 在脑 registry（encephalon）注册自己
3. 保持无状态 —— 对话历史从 `SkillContext` 流入、从 `SkillOutput.updated_history_summary` 流出，由调用方持久化
4. 把 LLM + TTS + ASR + Emotion provider 打包为一个 `ProviderSet`
5. 长历史可选走 `HistorySummarizer` 摘要

`synapse::lobe` 把这五件事打包好，让 lobe 作者只写 skill，不用碰 A2A plumbing。

## 解剖

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

## 快速开始

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

自动获得：

- A2A server（axum）监听所请求端口
- AgentCard 在启动时自动发到 encephalon registry
- `StatelessSkillRouter` 按 `skill_id` dispatch
- SIGHUP 时优雅 reload `LobeConfig`
- 接好 `lobe::metrics` 计数器

## 核心 traits

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

Skill 是一个 async 函数，输入是 typed `Input` + `SkillContext`，输出是 typed `Output`。**调用之间 skill 内部不持有状态** —— 对话历史通过 `SkillContext.history_summary` 流入、通过 `SkillOutput.updated_history_summary` 流出。

### `SkillContext`

```rust
pub trait SkillContext: Send + Sync {
    fn user_id(&self) -> &str;
    fn history_summary(&self) -> Option<&str> { None }
    fn system_prompt(&self) -> Option<&str> { None }
    fn extra(&self) -> Option<&serde_json::Value> { None }
}
```

伴随每次 skill 调用的自由 JSON context。`JsonCtx` 结构开箱即用地提供 `user_id` + `history_summary` + `extra`。

### `SkillOutput`

```rust
pub struct SkillOutput {
    pub updated_history_summary: Option<String>,
    pub payload: serde_json::Value,
    pub audio_url: Option<String>,
}
```

承载 skill 响应 payload + 可选音频输出 + 可选新历史摘要（由调用方持久化）。

### `EmotionProvider`

```rust
#[async_trait]
pub trait EmotionProvider: Send + Sync {
    async fn detect(&self, input: EmotionInput) -> EmotionOutput;
}
```

需要情绪感知响应的 lobe（比如陪伴 lobe）实现这个 trait；其他 lobe 可以忽略。返回 valence / arousal / dominance。

### `LobeEnv`

```rust
pub trait LobeEnv {
    fn bind_addr(&self) -> SocketAddr;
    fn registry_url(&self) -> &str;
    fn provider_set(&self) -> &ProviderSet;
    // ... override per-lobe
}
```

环境变量驱动的配置 trait：每个 lobe 加载自己的环境变量（`LOBE_BIND`、`LOBE_REGISTRY_URL`、`OPENAI_API_KEY` …）并产出 `LobeEnv` 实现。

## 助手

### `ProviderSet`

```rust
pub struct ProviderSet {
    pub llm: Arc<dyn LLMProvider>,
    pub tts: Option<Arc<dyn TtsProvider>>,
    pub asr: Option<Arc<dyn AsrProvider>>,
    pub emotion: Option<Arc<dyn EmotionProvider>>,
}
```

一个 lobe 所需全部 provider 的合集。在 `main.rs` 构造一次，便宜地克隆到每个 skill 里。

### `HistorySummarizer`

```rust
pub struct HistorySummarizer { /* … */ }

impl HistorySummarizer {
    pub fn new(llm: Arc<dyn LLMProvider>, max_tokens: usize) -> Self;
    pub async fn summarize(&self, history: &str) -> Result<String, String>;
}
```

当 `SkillContext.history_summary` 太长装不进模型 context 时使用的 LLM 摘要器。不需要长历史的 lobe 可以完全跳过。

### `StatelessSkillRouter`

```rust
let router = StatelessSkillRouter::new()
    .add("echo", Arc::new(EchoSkill))
    .add("summarize", Arc::new(SummarizeSkill));
```

把 `skill_id` 映射到 `Arc<dyn StatelessSkill>` 用于 A2A dispatch。`LobeApp.skill(...)` 自动接好。

## A2A 注册

`LobeApp.serve(...)` 启动 axum A2A server 并把 lobe 注册到 encephalon registry。需要更细粒度时：

```rust
use synapse::lobe::{build_a2a_router, register_with_encephalon, AgentCard};

let card = AgentCard::new("my-lobe", "My specialized lobe", "http://0.0.0.0:9101", "1.0");
let router = build_a2a_router(card.clone(), skill_router);
register_with_encephalon(&card, "http://encephalon:60080").await?;

axum::serve(tcp_listener, router).await?;
```

## `LobeApp` 构造器

| 方法 | 用途 |
|------|------|
| `LobeApp::new(name, description)` | 新建应用 |
| `.bind(addr)` | 覆盖绑定地址 |
| `.skill(Arc<dyn StatelessSkill>)` | 注册 skill |
| `.provider_set(ps)` | 注入自定义 ProviderSet |
| `.env(LobeEnv impl)` | 加载环境变量配置 |
| `.serve(addr)` | 启 axum + 注册 encephalon + 永久运行 |

## 何时用 `synapse::lobe`

| 场景 | 用 lobe? |
|------|:---:|
| 新的专门 agent 服务（音频 / 视觉 / 工作流） | ✅ |
| 一次性脚本或库调用方 | 直接用 `synapse::a2a::*` |
| 把 agent 逻辑嵌入另一个 Rust crate | 直接用 `synapse::adk::*` |
| WASM 沙箱 skill | ✅ via `WasmLobeRuntime` |
| 需要情绪的纯 Rust skill | ✅ |

## 相关阅读
