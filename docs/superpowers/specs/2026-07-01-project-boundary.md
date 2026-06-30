# Synapse Project Boundary

> **What Synapse is, what it isn't, and how downstream projects use it.**

## TL;DR

**Synapse** = a Rust framework for building **agent primitives and the
mechanisms that connect them** (streaming, A2A protocol, WASM package
plumbing, transport).

**Synapse explicitly does NOT do**:

- ❌ Agent supervision / scheduling / orchestration
- ❌ Cluster management / load balancing
- ❌ HTTP serving / API gateway concerns
- ❌ Persistence / state stores
- ❌ CLI / dashboard / user interfaces

Those concerns are owned by **downstream projects** (today: Prefrontal;
tomorrow: anything else).

## Layered architecture

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 4 — User-facing apps                                  │
│            shadow-mobile, shadow-desktop, shadow-cli         │
├──────────────────────────────────────────────────────────────┤
│  Layer 3 — Runtime / supervisor                              │
│            prefrontal  ·  other "supervisor" projects        │
│            (loads WASM agents, supervises, A2A-routes)       │
├──────────────────────────────────────────────────────────────┤
│  Layer 2 — Synapse (this project)                            │
│            streaming core · A2A SDK wrap · WASM runtime env  │
│            transport adapters · component primitives         │
├──────────────────────────────────────────────────────────────┤
│  Layer 1 — Foundation crates                                 │
│            wasmtime · a2a-rs · reqwest · wasi-http · axum   │
└──────────────────────────────────────────────────────────────┘
```

Layer 2 is Synapse; layers 1 and 3+ are somebody else's concern.

## What Synapse provides

| Module | Owned by Synapse | Notes |
|---|---|---|
| **`stream`** (`src/stream/`) | ✅ | `StreamReader<T>`, `StreamWriter<T>`, `CancelHandle`, generic over `T` |
| **`adk::stream`** | ✅ | `AgentStreamEvent`, `AgentStream` (the unified event bus) |
| **`components::model::delta`** | ✅ | `MessageDelta`, `FinishReason`, `Usage` |
| **`components::model::*`** | ✅ | `BaseChatModel` trait + concrete (OpenAI / Anthropic) impls |
| **`adk::agent`** | ✅ | `Agent` trait, `ChatModelAgent`, `GraphReActAgent` |
| **`adk::multiagent`** | ⚠️ | **In-scope as reference patterns**; Prefrontal-style supervisors replace these |
| **`transport`** | ✅ | `HttpClient` trait + `ReqwestHttpClient` / `WasiHttpClient` |
| **`transport::sse`** | ✅ | SSE parser + `HttpClientSse::sse_post` (native impl) |
| **`wasm`** (`src/wasm/`) | ✅ | `WasmPackage`, `WasmRuntime`, `AgentInstance`, registry, manager — reusable infrastructure |
| **`a2a`** | ✅ | Thin wrappers over `a2a-rs` SDK; protocol-only, no hosting concerns |

## What Synapse does NOT provide (deliberately)

| Concern | Why it's not in Synapse | Where it lives |
|---|---|---|
| Host an axum server | Prefrontal's job | `prefrontal/` |
| Spawn / supervise WASM instances | " | `prefrontal/src/supervisor.rs` |
| Persist task state across restarts | Storage choice is deployment-specific | `prefrontal/src/persistence/` |
| Schedule agents across machines | Cluster concern | A future `prefrontal-cluster/` or sibling project |
| UI / CLI / dashboard | App concern | `shadow-*` projects |
| Auth / multi-tenancy | Deployment concern | Layer 4 or Reverse proxy |
| LLM provider secrets at rest | Secrets handling is host concern | Prefrontal or operator |

## What an "Agent" looks like to a downstream project

The minimum a downstream project (Prefrontal) needs to do to host a
Synapse agent:

1. **Compile the agent crate to wasm32-wasip2** — see
   `docs/wasm-compilation-guide.md` for the step-by-step.
2. **Load it** via `synapse::wasm::WasmRuntime::load(package)`.
3. **Bind an A2A route** to it via `synapse::a2a::server::*` (the
   thin wrapper around `a2a-rs`).
4. **Forward outbound A2A calls** through `synapse::a2a::client::*`.

The downstream project owns the URL paths, the WebSocket upgrades, the
auth middleware, the task store — Synapse does not.

## Reusable contract for downstream projects

Two things must stay stable across versions, because multiple downstream
projects depend on them:

1. **WASM Agent ABI** — the function signatures a Synapse-compiled
   agent WASM exposes. See `docs/wasm-agent-abi.md`. **Breaking
   changes here require a major version bump.**

2. **A2A wire compatibility** — guaranteed by delegating to upstream
   `a2a-rs` v0.4.x. Pin tightly; track upstream minor bumps
   quarterly.

Everything else (internal trait design, helper modules) can change
freely across minor versions.

## On providing an "Agent template"

Two options were considered:

| Option | Pros | Cons |
|---|---|---|
| **(A) Template repo** (`synapse-agent-template`) | Easy to copy; clear example | Duplicate; drifts from Synapse |
| **(B) `synapse new` CLI / `synapse-sdk` sub-crate** | Stays current; blessed path | Requires maintaining a tool |

**Decision (proposed)**: ship **(B)** as a thin `cargo-generate`-style
template under `templates/agent/` in this repo (read-only skeleton
the user copies); skip the CLI for v1. The template pulls the agent
into a workspace with `Cargo.toml` already configured for
`wasm32-wasip2` + the WASM Agent ABI exports documented in
`docs/wasm-agent-abi.md`.

The **agent template** is the user-facing "hello world" of building a
Synapse agent. It compiles to one WASM file Prefrontal (or any other
layer-3 host) can load.

## On the docs split

| Document | Owner |
|---|---|
| `docs/howto/*.md` — how to call each component | **Synapse** |
| `docs/reference/*.md` — API reference | **Synapse** |
| `docs/wasm-agent-abi.md` — WASM exports | **Synapse** |
| `docs/wasm-compilation-guide.md` — `cargo build --target wasm32-wasip2` | **Synapse** |
| "How Prefrontal uses Synapse" / "Architecture" | **Prefrontal** (their docs) |
| "How shadow-mobile embeds Prefrontal" | **shadow-mobile** (their docs) |

This keeps Synapse documentation strictly about *how to use the API*,
not about *what the deployment topology looks like*.

## Future-proofing

Other projects in the family (any sibling "supervisor" or "host"
project — call them **layer-3 projects**) should all be able to:

- Include `synapse` as a Cargo dependency
- Use `synapse::wasm::WasmRuntime` to load any Synapse-compiled agent
- Use `synapse::a2a::*` to wire up A2A protocol handling
- Use `synapse::stream::*` to consume agent event streams

Synapse will track this via:

- **Sematic Versioning**: pre-1.0, expect minor versions to break
  the WASM ABI; we'll bump major (e.g. 0.2.x → 1.0) the moment we
  commit to stable ABI.
- **Crate features**: optional sub-modules (e.g. `embeddings`,
  `rag`, `mcp`) so downstream projects only compile what they use.
- **Doc-test coverage**: every public API in `docs/reference/`
  should have a runnable example as a `///` docstring.
