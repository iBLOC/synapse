# WASM Sandbox

Synapse is designed to compile to both **native** and **WASM**
targets, enabling deployment in sandboxed environments (e.g., PFC
runtime).

## Cross-Platform Transport

The `HttpClient` trait abstracts HTTP, with two implementations:

| Target | Implementation | Use |
|--------|----------------|-----|
| Native (`cfg(not(target_arch = "wasm32"))`) | `ReqwestHttpClient` (reqwest) | Direct HTTP calls |
| WASM (`cfg(target_arch = "wasm32")`) | `WasiHttpClient` (wasi::http) | HTTP via host function |

The choice is made at compile time via `type DefaultHttpClient = ...`.

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = native::ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = wasi::WasiHttpClient;
```

## Building for WASM

```bash
# Add wasm32-wasip2 target
rustup target add wasm32-wasip2

# Build
cargo build --target wasm32-wasip2 --features wasm
```

The `wasm` feature adjusts dependencies:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1", features = ["sync", "macros", "io-util", "rt", "time"] }
wasi = "0.14"
pollster = "0.4"
```

## wasi::http Bridge

`WasiHttpClient` bridges async Rust to wasi's poll-based HTTP API:

```rust
// wasi::http is poll-based; tokio is async
// WasiHttpClient uses spawn_blocking + pollster to bridge
async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
    tokio::task::spawn_blocking(move || {
        pollster::block_on(wasi_async_send(req))
    })
    .await
    .map_err(|e| HttpError::Request(e.to_string()))?
}
```

## Use Case: PFC Runtime

PFC (a sibling project) loads Synapse-compiled agents into a
`wasmtime` sandbox:

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
the PFC host's controlled `wasi::http` implementation, which can
enforce auth, rate limits, and audit logging.

## See Also

- [WASM Deploy how-to](../howto/wasm-deploy.md)
- [A2A Protocol](./a2a.md)
