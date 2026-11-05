# Synapse — 神经突触层 (Shared Foundation Library)

**Updated**: 2026-11-06 (OpenZiti v2 线收口 —— **真网 5 条 #[ignore] 全绿**：`cargo test --offline --features ziti --lib -- --ignored --nocapture` ⇒ `5 passed; 0 failed; finished in 6.48s`，EXIT=0（quickstart router @ 192.168.31.170 在线；client/host 身份在 shadow/.ziti/；宿主走生产 serve 入口 example `ziti_v2_brain`；逐条读数：HTTP 200＋体对 / WS 101＋回帧 / SSE 200＋text/event-stream＋分帧 / 本地代理 200＋体对 / 整链 overlay 自宿主自客户端一条命令全绿）；lib 测试 **371 passed / 0 failed / EXIT=0**，本线 3 处 warning 清零（live_tests 两处多余 mut ＋一处未用 import）；手册 `docs/concepts/ziti.md` 与本文件 ziti 段改写为 v2 自建叙事（第三方 ziti-sdk v1 已删出依赖图；`../ziti-rust` crates/ziti-v2 path 依赖；守门员闸 `check-ziti-v2-wire.mjs` ＋ `check-ziti-face.mjs` 全绿、阻断 0 项）);
2026-11-01 (synapse 库级默认监听改回环 —— 宽绑定必须显式；两条理由，缺一条都不够： ① **暴露面**（默认安装不该在局域网可读）； ② **形状**（`build()` 把 card 的 URL 直接从**监听地址**拼出来 ⇒ ⇒ 宽绑定 ⇒ ⇒ card 报 `http://0.0.0.0:8090` ⇒ ⇒ `0.0.0.0` **不是一个可连接的地址** ⇒ ⇒ 「注册成功、永远连不上」 —— 又一个「失败长得像成功」）； 新增 `lobe_app::DEFAULT_LOBE_BIND`（`127.0.0.1:8090`）与 `a2a::server::DEFAULT_A2A_BIND` （`127.0.0.1:8080`）两个常量 ⇒ ⇒ 宽绑定只能显式（`LOBE_BIND` / `.bind(…)`）； 证据：`cargo test --lib` **336 ⇒ 338 passed / 0 failed**（两条新守卫）⇒ ⇒ 反向验证 （两个常量改回宽绑定 ⇒ ⇒ **恰好 2 红**、其余 336 仍绿）⇒ ⇒ 父仓闸 `check-bind-exposure.mjs --lobes` **0 条 / EXIT=0**（默认档也 0 条）；提交 `1046ff7`);
2026-11-01 (「监听地址」与「公布地址」分成两个槽 —— 病根是**一个字段说两件事**：宽绑定时监听地址是 `0.0.0.0:19500`，而 `0.0.0.0` **不是一个可连接的地址**（它是「监听所有网卡」）⇒ ⇒ 单机上碰巧能通、换台机器永远连不上 ⇒ ⇒ 「注册成功、永远连不上」（又一个「失败长得像成功」）⇒ ⇒ 一份解析器落在 `synapse::net::bind`（`parse_bind` / `resolve_listen_and_public` / `sanitize_public_url` / `public_url_for_base`；端脑仓那份改成**委托**，路径不变 ⇒ ⇒ **别各写一套** —— 每个叶区仓都依赖 synapse、却都没有端脑依赖 ⇒ ⇒ 放端脑仓叶区就**用不上**）；⇒ ⇒ 叶区注册通道（`lobe::register_with_encephalon`）请求体多一个 `public_url` 槽（显式 `LOBE_PUBLIC_URL` 优先，没给就把监听地址**洗成可连接的**）；⇒ ⇒ ⚠️⚠️ 顺手逮到一条**更老的病**：**改名收口漏了共享库** —— 这个函数一直打在**老路径** `/encephalon/lobes/{name}`，而端脑 v0.3 只挂 `/telencephalon/lobes/{name}` ⇒ ⇒ **404** ⇒ ⇒ 注册**静默失败**（叶区只 `warn!` 一行）⇒ ⇒ 颞叶/顶叶/枕叶/前额叶**从来没注册成功过** ⇒ ⇒ 「端脑知道自己有哪些叶区」有一半是自己种的占位 ⇒ ⇒ 现在**先打新路径**、**只有 404** 才回落到老路径一次（401/403 **不**回落 ⇒ ⇒ 不掩盖鉴权问题）; 2026-10-30 (A2A 工具事件通道 `a2a::tool_event` —— 工具调用以前过不了 A2A 这一跳：官方 `StreamResponse` 只有 `Task`/`Message`/`StatusUpdate`/`ArtifactUpdate` 四个变体、**没有任何一类能承载工具调用**，服务端把 `ToolStart` 拍平成纯文本 `Calling tool: name(args)`、客户端又把不是失败的 `StatusUpdate` **全部丢弃** ⇒ 工具事件在线上被消灭，下游（如 telencephalon 端脑的 `tool_call` 分支）永远不可达。修法：用 `StatusUpdate.metadata` 开一条**结构化旁路**（键 `synapse_tool_event`，值 `{kind: start|call|end, ...}`），服务端写、客户端还原回 `AgentStreamEvent`；**纯文本照样保留** ⇒ 老对端行为不变、线上没删任何东西。反向验证：短路客户端还原 ⇒ 圆跳测试 `a2a::server::tests::tool_call_survives_the_full_a2a_hop` FAILED（`left: 0, right: 1`）); 2026-10-11 (two-locale docs site EN root + `docs/zh/`, copy-as-Markdown + `llms.txt` via postbuild, `examples/` runnable samples; `synapse::acp` client module — drives external ACP agents via the official `agent-client-protocol` 2.2 SDK per `1429a95`); 2026-09-25 (`transport::ziti` per `8ff2294` (2026-09-09), audio providers per `cdb6249` (2026-08-22); A2A metadata wire-forwarding via `__encephalon_meta_v1__` per `135f1fd`; WASM-A2A bridge sync path; zero-warnings pass prior baseline 2026-07-19)
**Status**: ✅ v0.4 lobe module complete + WASM-A2A sync bridge + A2A metadata round-trip via `__encephalon_meta_v1__` + `transport::ziti` zero-trust A2A over OpenZiti + `acp` client driving external ACP agents (opencode / claude-code-acp / codex-acp)

## Role

Shared foundation library for the Shadow platform. Provides A2A protocol, WASM runtime, LLM providers, and the new `synapse::lobe` module for stateless skill lifecycle.

## Architecture

```
synapse/src/
├── a2a/                    # Agent-to-Agent protocol (card, client, server)
├── acp/                    # ACP client (opencode / claude-code-acp / codex-acp via official agent-client-protocol 2.2 SDK; added 2026-09-26)
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
cargo build                    # default — no acp, no ziti, no audio
cargo build --features acp     # opt in to ACP client integration
```

## Test

```bash
cd synapse
cargo test --lib lobe          # lobe module tests
cargo test --lib acp           # ACP client (requires --features acp)
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

### Defaults（默认监听 ⇒ 回环）
- `lobe_app::DEFAULT_LOBE_BIND` = `127.0.0.1:8090` —— `LobeApp::build()` 的默认监听
- `a2a::server::DEFAULT_A2A_BIND` = `127.0.0.1:8080` —— `serve_with_store()` 的默认监听
- ⚠️ **宽绑定必须显式**（`LOBE_BIND=0.0.0.0:PORT` 或 `.bind(…)`）⇒ ⇒ 两条理由，缺一条都不够：
  ① **暴露面** —— 默认安装不该在局域网可读；
  ② **形状** —— `build()` 把 card 的 URL 直接从**监听地址**拼出来 ⇒ ⇒ 宽绑定 ⇒ ⇒ card 报
     `http://0.0.0.0:8090` ⇒ ⇒ `0.0.0.0` **不是一个可连接的地址** ⇒ ⇒ 「注册成功、永远连不上」。
- ⚠️ 想跨机 ⇒ **两个都要显式**：监听地址 + 公布地址（`LOBE_PUBLIC_URL`；宽绑定时自动回落回环）。

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
- ziti-v2 (path dep `../ziti-rust/crates/ziti-v2`) — **optional**, behind `feature = "ziti"` (zero-trust A2A over OpenZiti; self-built **v2** wire layer — third-party ziti-sdk v1 removed from the dep graph 2026-11-04)
- agent-client-protocol 2.2 — **optional**, behind `feature = "acp"` (drives external ACP agents over stdio)

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

### Wire layer (v2, self-built since 2026-11-04)

OpenZiti ships official SDKs for Go / C / Swift / Java / Python /
C# but **not Rust**. The old third-party `ziti-sdk` crate (v1
protocol) has been **removed from the dependency graph** — it
cannot connect to a v2 controller. The wire layer is now **ours**:
`ziti-v2` in the sibling repo `../ziti-rust` (`crates/ziti-v2`),
wired in as a path dependency. synapse keeps only a re-export
facade (`src/transport/ziti/v2/mod.rs`).

Guard rails: parent-repo gate `check-ziti-v2-wire.mjs` pins the
protocol constants / wire invariants / live-test entry points;
live proof in `src/transport/ziti/live_tests.rs` (5 `#[ignore]`d
tests over a real network) read **5 passed / 0 failed in 6.48s**
on 2026-11-06 over the quickstart router.

### Migration posture

The module is insulated from callers by the `ZitiGateway` /
`ZitiListener` / `ZitiStream` API. That insulation survived the
v1 → v2 swap (and the move to the `ziti-rust` repo) with zero
downstream changes; future wire swaps stay local to `ziti-v2` +
`gateway.rs` / `listener.rs`.

## `acp` Client module (added 2026-09-26)

Client-only integration that drives external coding agents speaking the
[Agent Client Protocol][acp] (opencode, claude-code-acp, codex-acp,
…) over stdio and surfaces their output as a normal Synapse
[`AgentStreamEvent`] stream. Implements the ACP *Client* role inside
Synapse; speaking the ACP *Agent* role is out of scope for this module.

[acp]: https://agentclientprotocol.com/

Public surface:
- `AcpAgentSpec::opencode()` / `claude_code()` / `codex()` — built-in
  presets for the three reference ACP shims; or `AcpAgentSpec::new(name, cmd)`
  + `.arg(...)` / `.env(k, v)` for fully custom commands.
- `AcpClient::connect(spec)` — spawn the agent subprocess, run the
  `initialize` handshake, start a long-lived dispatch task. Returns
  once the handshake completes; the task keeps running for the
  lifetime of the client.
- `client.session().prompt(text)` — send one prompt; returns a
  `StreamReader<AgentStreamEvent>`. Each call opens a fresh ACP
  session (`session/new`) on the shared connection — connection and
  initialize handshake are reused across prompts.
- `AcpError::Sdk` / `AcpError::Cancelled` — the error enum.

Wire model:

```text
 ┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
 │ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
 │ AcpClient      │                        │ (external subprocess)   │
 └────────────────┘                        └────────────────────────┘
```

The SDK spawns the subprocess for us (via `AcpAgent::new(AcpAgentConfig)`);
we never touch `tokio::process::Command` directly. Each `AcpSessionHandle`
owns one ACP session over one stdio transport.

### Why the official SDK

Four correctness traps we explicitly avoided by depending on
`agent-client-protocol` instead of hand-rolling a JSON-RPC client:

1. **Protocol version negotiation** — the SDK downgrades/refreshes based
   on `InitializeResponse.protocol_version`. A hand-rolled client would
   freeze on `V1` and silently break against v2 agents.
2. **Request/response correlation** — `SentRequest::block_task().await`
   plus internal request-id bookkeeping eliminates the "which response
   belongs to which request" race. DIY clients almost always get this
   wrong eventually.
3. **`$/cancelRequest` semantics** — UI cancel round-trips correctly
   even when a proxy sits in front. DIY clients typically forget the
   JSON-RPC cancel notification and leak subprocess work after stop.
4. **stdio framing** — `Stdio` handles `Content-Length` headers vs
   line-delimited frames across agents. `AsyncBufReadExt::lines()` does
   not.

Bonus forward-compatibility the SDK buys us for free:
- `Conductor` / `Proxy` roles in `role::acp` are ready for multi-agent
  fan-out; we don't need to redesign the connection layer when
  shadow-desktop starts driving opencode + claude-code-acp side-by-side.
- `with_mcp_server` lets Synapse inject its own MCP servers into a
  session — agents call our tools without us opening a second port.
- `SessionUpdate` is `#[non_exhaustive]`, which forces
  `translate_update` to go through `serde_json` round-trip. New schema
  variants fall through to the `_ => Vec::new()` arm instead of breaking
  compilation.

### SDK caveat

ACP ships an official SDK for Rust — unlike OpenZiti — so there is no
vendor-strategy caveat here. The crate (`agent-client-protocol`, Apache-2.0)
is the official binding maintained at [agentclientprotocol/rust-sdk][sdk].
Pin to `2.2` for now; bump on minor releases only after reading the
changelog, since the public API is still pre-1.0.

[sdk]: https://github.com/agentclientprotocol/rust-sdk

### Build & test

Gated by `feature = "acp"` (default off) and **non-wasm only** — the
upstream SDK does not expose `AcpAgent` / `Stdio` on `target_arch =
"wasm32"`.

```bash
cargo build  -p synapse --features acp
cargo test   -p synapse --features acp --lib acp
```

Nine unit tests cover `AcpAgentSpec` builder behavior, the
`SessionUpdate → AgentStreamEvent` translator (Pending/InProgress →
`ToolStart`, Completed/Failed → `ToolEnd`, `AgentMessageChunk` →
`TokenDelta`), and the `SessionNotification` envelope parser
(envelope / bare-update / unparsable / round-trip).

### Migration posture

Callers depend only on `AcpClient` / `AcpSessionHandle` / `AcpAgentSpec`
/ `AcpError`. Wire-level types (`SessionUpdate`, `ToolCallStatus`, …)
are hidden behind `translate.rs`, so a future SDK major bump (v3) that
renames or restructures those types is a localized change to the
translator — public API stays put.

### Known gaps (future work)

- **Outbound `Dispatch::Request`** from the agent (e.g. fs/read) is
  answered with a generic `InternalError`. When the time comes to let
  agents call back into Synapse, plug `Builder::on_receive_request`
  into the `connect_with` chain.
- **Capability advertisement** — we always send `ProtocolVersion::V1`
  regardless of what the agent advertised. If we start depending on v2
  features (`V2SessionBuilder`, MCP-over-ACP), switch the
  `Client::default()` to `Client::default().v2()` so negotiation
  fails loud when the agent is too old.

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

## Documentation / 文档索引

The docs site is a **two-locale VitePress tree**: English pages at the site root, 简体中文 pages under `docs/zh/` mirroring the same paths. VitePress `locales` in `docs/.vitepress/config.ts` renders the navbar language switcher; nav/sidebar labels are translated per locale. Content pages exist in BOTH trees — when you add or change a page under `docs/guide|concepts|howto|reference/` or `docs/index.md`, write the Chinese counterpart under `docs/zh/<same-path>` in the same change. Code blocks, identifiers and `/api/*` (rustdoc) links are shared and stay untranslated; internal site links in `zh/` pages are prefixed with `/zh`. Excluded from translation (stay English-only / root-only): `docs/CHANGELOG.md` (release log, high churn), `docs/SECURITY.md` / `docs/CONTRIBUTING.md` (1-line stubs), the legacy Chinese-primary guide pages (`framework-alignment`, `lcel-pipe`, `memory`, `observability`, `opentelemetry-*`, `structured-output`, `typed-lambda`) and `docs/superpowers/` (internal specs, not in nav).

**AI-friendliness is a build feature**: `docs/scripts/postbuild.mjs` (chained into `npm run build` / `build:pages`) copies every source `.md` into `dist/` at its original path (stable raw-markdown URLs — `https://ibloc.github.io/synapse/<path>.md`), and generates `llms.txt` (curated index) + `llms-full.txt` (full corpus) at the site root. The theme adds a "Copy page as Markdown" button under every page (`docs/.vitepress/theme/components/DocActions.vue`). Keep that script in the build chain when touching `docs/package.json`.

**Runnable examples** live in `examples/*.rs` at the repo root and are compiled by `cargo build --examples` — they are the executable source of truth for how to call the public API. When a how-to page's code block and an example disagree, the example (compiler-checked) wins; fix the docs. The Examples nav page (`docs/guide/examples.md` + `docs/zh/guide/examples.md`) indexes them.

Concept docs under `docs/concepts/` are the deep-dive for each major module. Concept 文档在 `docs/concepts/` 下，每个核心模块一份：

- [`docs/concepts/a2a.md`](docs/concepts/a2a.md) — A2A protocol (client + server + TaskStore trait) / A2A 协议（client + server + TaskStore trait）
- [`docs/concepts/acp.md`](docs/concepts/acp.md) — `synapse::acp` client driving opencode / claude-code-acp / codex-acp / 驱动 opencode / claude-code-acp / codex-acp
- [`docs/concepts/lobe.md`](docs/concepts/lobe.md) — `synapse::lobe` v0.4 stateless skill lifecycle glue / 无状态 skill 生命周期 glue
- [`docs/concepts/ziti.md`](docs/concepts/ziti.md) — `transport::ziti` zero-trust A2A over OpenZiti / 零信任 A2A over OpenZiti
- [`docs/concepts/architecture.md`](docs/concepts/architecture.md) — full module tree / 全模块树
- [`docs/concepts/mcp.md`](docs/concepts/mcp.md) — Model Context Protocol (client + server) / Model Context Protocol（client + server）
- [`docs/concepts/wasm.md`](docs/concepts/wasm.md) — WASM agent ABI v1 + WasiHttpClient / WASM agent ABI v1 + WasiHttpClient
- [`docs/concepts/multiagent.md`](docs/concepts/multiagent.md) — Router / Orchestrator / Handoff / Hybrid / 四种模式
- [`docs/concepts/agent.md`](docs/concepts/agent.md) · [`model.md`](docs/concepts/model.md) · [`tool.md`](docs/concepts/tool.md) · [`document-rag.md`](docs/concepts/document-rag.md) · [`llm-retry-policy.md`](docs/concepts/llm-retry-policy.md) · [`session-event-log.md`](docs/concepts/session-event-log.md) · [`tool-namespace.md`](docs/concepts/tool-namespace.md) · [`waterfall-events.md`](docs/concepts/waterfall-events.md) — remaining concept docs / 其余概念文档

Bilingual README at root: [`README.md`](README.md) (zh primary + en mirror / 中文主 + 英文镜像).

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
| 1429a95 | feat(acp): add `synapse::acp` client module — drive external ACP agents (opencode / claude-code-acp / codex-acp) over stdio via the official `agent-client-protocol` 2.2 SDK; gated by `feature = "acp"` (off by default, non-wasm only) |
