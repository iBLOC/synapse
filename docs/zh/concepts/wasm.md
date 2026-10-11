---
title: WASM Sandbox
---

# WASM 沙箱

Synapse 设计为可同时编译到 **native** 与 **WASM** 目标，从而能部署进沙箱环境（例如端脑 Telencephalon runtime）。

## 跨平台 Transport

`HttpClient` trait 对 HTTP 做了抽象，提供两个实现：

| 目标 | 实现 | 用途 |
|--------|----------------|-----|
| Native (`cfg(not(target_arch = "wasm32"))`) | `ReqwestHttpClient` (reqwest) | 直接发起 HTTP 调用 |
| WASM (`cfg(target_arch = "wasm32")`) | `WasiHttpClient` (wasi::http) | 经 host function 走 HTTP |

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = native::ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = wasi::WasiHttpClient;
```

## 面向 WASM 构建

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --features wasm
```

`wasm` feature 会调整依赖：

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1", features = ["sync", "macros", "io-util", "rt", "time"] }
wasi = "0.14"
pollster = "0.4"
```

## 使用场景：端脑（Telencephalon）Runtime

端脑（Telencephalon，姊妹项目）把编译成 WASM 的 Synapse agent 托管进 `wasmtime` 沙箱：

```
┌─────────────────────────────────────┐
│  Telencephalon Host                 │
│   ├─ wasmtime runtime               │
│   │   └─ Synapse agent (WASM)       │
│   │        └─ WasiHttpClient        │
│   │             ↓ (host function)   │
│   ├─ HTTP server (real reqwest)     │
│   └─ LLM proxy / tool registry      │
└─────────────────────────────────────┘
```

WASM agent 没有直接的网络访问。所有 HTTP 都经由端脑（Telencephalon）host 受控的 `wasi::http` 实现。

## 相关阅读
