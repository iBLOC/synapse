---
title: WASM Sandbox
---

# WASM Sandbox · WASM 沙箱

Synapse is designed to compile to both **native** and **WASM**
targets, enabling deployment in sandboxed environments (e.g., PFC
runtime).
*Synapse 设计为可同时编译到 **native** 与 **WASM** 目标，从而能部署进沙箱环境（例如 PFC runtime）。*

## Cross-Platform Transport · 跨平台 Transport

The `HttpClient` trait abstracts HTTP, with two implementations:
*`HttpClient` trait 对 HTTP 做了抽象，提供两个实现：*

| Target / 目标 | Implementation / 实现 | Use / 用途 |
|--------|----------------|-----|
| Native (`cfg(not(target_arch = "wasm32"))`) | `ReqwestHttpClient` (reqwest) | Direct HTTP calls / 直接发起 HTTP 调用 |
| WASM (`cfg(target_arch = "wasm32")`) | `WasiHttpClient` (wasi::http) | HTTP via host function / 经 host function 走 HTTP |

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = native::ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = wasi::WasiHttpClient;
```

## Building for WASM · 面向 WASM 构建

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --features wasm
```

The `wasm` feature adjusts dependencies:
*`wasm` feature 会调整依赖：*

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1", features = ["sync", "macros", "io-util", "rt", "time"] }
wasi = "0.14"
pollster = "0.4"
```

## Use Case: PFC Runtime · 使用场景：PFC Runtime

PFC (a sibling project) loads Synapse-compiled agents into a
`wasmtime` sandbox:
*PFC（姊妹项目）把编译成 WASM 的 Synapse agent 加载进 `wasmtime` 沙箱：*

```
┌─────────────────────────────────────┐
│  PFC Host                           │
│   ├─ wasmtime runtime               │
│   │   └─ Synapse agent (WASM)       │
│   │        └─ WasiHttpClient        │
│   │             ↓ (host function)   │
│   ├─ HTTP server (real reqwest)     │
│   └─ LLM proxy / tool registry      │
└─────────────────────────────────────┘
```

The WASM agent has no direct network access. All HTTP goes through
the PFC host's controlled `wasi::http` implementation.
*WASM agent 没有直接的网络访问。所有 HTTP 都经由 PFC host 受控的 `wasi::http` 实现。*

## See Also · 相关阅读

- [WASM Deploy how-to](/howto/wasm-deploy)
- [A2A Protocol](/concepts/a2a)
