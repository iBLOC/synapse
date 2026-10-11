---
title: transport::ziti
---

# `transport::ziti`

Zero-trust A2A over [OpenZiti][openziti]. Public IPs are never exposed — every service is a *dark service* accessible only via an OpenZiti identity that the controller has authorised.

The module is gated behind the `ziti` Cargo feature so callers that don't need OpenZiti pay no compile-time cost.

```toml
synapse = { path = "../synapse", features = ["ziti"] }
```

[openziti]: https://openziti.io/

## Why a transport module

Zero-trust networking is a **platform-level security primitive**. Centralising it here means every agent (encephalon, tiangong-*, third-party) uses the same wire, identity model, and dark-service lifecycle — instead of each caller maintaining its own `XxxDarkService` wrapper.

It is the canonical transport for A2A when the public network is untrusted (multi-cloud, edge gateways, on-prem ↔ cloud). Use it **instead of** mTLS-over-LAN, not in addition.

## Public surface

| Item | Type | Purpose |
|------|------|------|
| `ZitiGateway` | `struct` | Load an OpenZiti identity JWT, get a context |
| `ZitiGateway::dial(service_name)` | `async fn -> Result<ZitiStream, ZitiError>` | Peer connection to a service |
| `ZitiGateway::listen(service_name)` | `async fn -> Result<ZitiListener, ZitiError>` | Bind a dark service |
| `ZitiListener` | `struct` | Accept incoming `ZitiStream`s on a dark service |
| `ZitiListener::accept()` | `async fn -> Result<ZitiStream, ZitiError>` | Pull one incoming connection |
| `serve(router, identity_path, service_name)` | `async fn` | **One-call convenience** — load identity + register dark service + accept loop + serve each accepted `ZitiStream` as an axum `Router` (hyper over Ziti) |
| `serve_ziti_stream(stream, router)` | `async fn` | Serve one axum `Router` on one accepted `ZitiStream` (in-process glue: hyper over Ziti) |
| `ZitiHttpClient` | `struct` | Outbound HTTP through Ziti |
| `ConnectionPool` | `struct` | Reusable Ziti streams |
| `ZitiConfig` | `struct` | JSON config (loaded from controller-side identity file) |
| `ConfigWatcher` | `struct` | Watch config file changes |
| `ZitiStream` | re-export from `ziti-sdk` | One bidirectional Ziti connection |
| `ZitiError` | `enum` | Identity / connection / listen / IO errors |

## Quick start (most callers)

Replace the four-line `TcpListener` dance with a single call:

```rust,no_run
use synapse::transport::ziti;

# async fn run(router: axum::Router) -> anyhow::Result<()> {
// Was:
//   let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//   axum::serve(listener, router).await?;
// Now (zero-trust, public IP invisible):
ziti::serve(router, "/etc/encephalon/identity.json", "encephalon").await?;
# Ok(()) }
```

The identity file is a JSON document exported from your OpenZiti controller. The service name must already be configured on the controller with the right policy attached.

## Outbound HTTP through Ziti

```rust
use synapse::transport::ziti::{ZitiGateway, ZitiHttpClient};

let gateway = ZitiGateway::new("/etc/my-agent/identity.json").await?;
let client = ZitiHttpClient::new(gateway);

let resp: HttpResponse = client
    .get("http://encephalon.shadow/a2a/skills/echo")  // dark-service URL
    .send()
    .await?;

println!("status: {}", resp.status);
```

`ZitiHttpClient` is built on the same `HttpClient` trait as `ReqwestHttpClient` and `WasiHttpClient` so higher-level code (A2A client, MCP client, OpenAI provider) works identically regardless of which transport sits underneath.

## Feature gate

The entire module is wrapped in `#[cfg(feature = "ziti")]`. Opening the feature pulls in `ziti-sdk` + hyper glue; without the feature the module is absent from the build graph entirely.

```bash
cargo build  -p synapse --features ziti
cargo test   -p synapse --features ziti --lib transport::ziti
```

## SDK caveat — read this before depending on it

⚠️ **OpenZiti does not publish an official Rust SDK**. The `openziti` GitHub org maintains official SDKs for Go / C / Swift / Java / Python / C# but **not Rust**.

This module currently wraps the **third-party `ziti-sdk` crate** (`jaschadub/ziti-rust`, MIT, version 0.3.x). That crate is the only Rust binding to OpenZiti currently published on crates.io and it is **community-maintained** — not by the OpenZiti project.

### Mitigation strategy

Callers are insulated from SDK churn by the `ZitiGateway` / `ZitiListener` / `ZitiStream` API surface — switching to a different Rust binding (or to OpenZiti's official C SDK via FFI) only requires changes to `gateway.rs` and `listener.rs`. The `serve()` / `serve_ziti_stream()` / `ZitiHttpClient` surface stays stable.

If/when the OpenZiti project ships an official Rust SDK, the upgrade path is:

1. Bump the `ziti-sdk` dep in this crate's `Cargo.toml`
2. Re-implement `gateway.rs` + `listener.rs` against the new SDK's identity / context / connection types
3. Re-export `ZitiStream` from the new SDK (or wrap it) so `ZitiHttpClient` and `serve_ziti_stream` keep working unchanged

### Why we don't use the C SDK via FFI today

The OpenZiti project does ship an official `ziti-sdk-c` with an Apache-2.0 license. Switching to it via FFI was considered and rejected for now:

- C-FFI in async Rust adds unsafe-boundary complexity (lifetime management, manual `Send`/`Sync` impl, blocking-call → async adapter).
- The third-party Rust SDK implements the same wire protocol against the same OpenZiti controllers, so wire compatibility is preserved.
- The `ZitiGateway` abstraction means a future FFI rewrite is a local change, not an ecosystem-wide migration.

### Vendor strategy (recommended for production)

Treat the upstream `jaschadub/ziti-rust` crate as **untrusted third-party code** even though it's pinned in `Cargo.lock`:

- Fork it under your org's GitHub namespace
- Vendor it via `[patch.crates-io]` in a workspace-level `Cargo.toml` so a vendored copy is always reachable
- Track upstream via GitHub watches; the wire protocol itself is stable per OpenZiti's documented spec, so a frozen vendor fork will continue to work against newer controllers as long as the protocol version is still supported

## Migration posture

The module is insulated from callers by the `ZitiGateway` / `ZitiListener` / `ZitiStream` API. Swapping to a different SDK (including an eventual official Rust binding) is a local change to `gateway.rs` + `listener.rs`; downstream code is unaffected.

## When to use `transport::ziti`

| Scenario | Use ziti? |
|------|:---:|
| A2A across untrusted networks (multi-cloud, edge ↔ cloud) | ✅ |
| Service with strict zero-trust posture | ✅ |
| LAN-only deployment with mTLS already in place | ❌ mTLS-over-TCP is simpler |
| Public-facing service that genuinely needs a public IP | ❌ ziti's whole point is hiding the IP |
| WASM sandboxed lobe calling other agents | ❌ use `WasiHttpClient` |
| Browser-facing traffic | ❌ ziti is not a replacement for TLS+CDN |

## See also

- [`src/transport/ziti/mod.rs`](../../src/transport/ziti/mod.rs) — module-level rustdoc (full SDK caveat + vendor strategy)
- [`src/transport/ziti/gateway.rs`](../../src/transport/ziti/gateway.rs) — `ZitiGateway` impl
- [`src/transport/ziti/listener.rs`](../../src/transport/ziti/listener.rs) — `ZitiListener` impl
- [`src/transport/ziti/dark_service.rs`](../../src/transport/ziti/dark_service.rs) — `serve()` + `serve_ziti_stream()`
- [`src/transport/ziti/http_client.rs`](../../src/transport/ziti/http_client.rs) — `ZitiHttpClient` impl
- [`src/transport/mod.rs`](../../src/transport/mod.rs) — `HttpClient` trait (parent of all transports)
- [OpenZiti][openziti] — official project site
- [`docs/concepts/a2a.md`](a2a.md) — A2A protocol reference
