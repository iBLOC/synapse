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
| `ZitiStream` | re-export from `ziti-v2` | One bidirectional Ziti connection (self-built v2 wire layer) |
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

The entire module is wrapped in `#[cfg(feature = "ziti")]`. Opening the feature pulls in the self-built `ziti-v2` wire layer (path dep `../ziti-rust/crates/ziti-v2`) + hyper glue; without the feature the module is absent from the build graph entirely.

```bash
cargo build  -p synapse --features ziti
cargo test   -p synapse --features ziti --lib transport::ziti
```

## Wire layer — ours (OpenZiti v2, self-built)

⚠️ **OpenZiti does not publish an official Rust SDK**. The `openziti` GitHub org maintains official SDKs for Go / C / Swift / Java / Python / C# but **not Rust**.

**Since 2026-11-04 the wire layer is ours.** The third-party `ziti-sdk` crate (`jaschadub/ziti-rust` 0.3.x) spoke the **v1** wire protocol (WebSocket + home-made frames, no magic) — against an OpenZiti **v2** controller it simply cannot connect. It has been **removed from the dependency graph**.

The v2 implementation lives in its own repo, `../ziti-rust` (`crates/ziti-v2`), wired in as a **path dependency** (iBLOC is a multi-repo path workspace):

- channel-v5 binary framing (magic `03 06 09 0c`, little-endian, 20-byte header)
- content types 60783+ (`Connect` / `StateConnected` / `Data` / `Dial` / `Bind` / …)
- controller API: authenticate / list services / create service sessions
- `Connect` (60783) and `Bind` (60790) both answered by `StateConnected` (60784)
- `ZitiStream` — `AsyncRead + AsyncWrite` over the overlay

synapse keeps only a thin re-export facade (`src/transport/ziti/v2/mod.rs` = `pub use ziti_v2::*;`), so caller paths are unchanged. The `ZitiGateway` / `ZitiListener` / `serve()` / `ZitiHttpClient` surface stays stable — that insulation survived the v1 → v2 swap with zero downstream changes, which is exactly what it was designed for.

### Guard rails

- The parent repo's `scripts/check-ziti-v2-wire.mjs` pins the protocol constants, the three wire invariants, backpressure symmetry, and the live-test entry points — the gate that stops the constants from silently drifting or v1 from sneaking back.
- **Live proof** lives in [`src/transport/ziti/live_tests.rs`](../../src/transport/ziti/live_tests.rs): 5 `#[ignore]`d end-to-end tests over a real network (HTTP / WebSocket / SSE / local proxy / full overlay self-hosted). Run them when a real OpenZiti network is up:

### Local test runbook

Three layers, cheapest first. Commands below are verified verbatim against the source (env-var names read from `config.rs` / `live_tests.rs` / the host example, not from memory).

#### 1. Structure gates (shadow repo, no network needed)

```powershell
node scripts/check-ziti-v2-wire.mjs            # protocol constants + wire invariants + live-test entry points
node scripts/check-ziti-face.mjs               # eleven wiring rules across the four repos
node scripts/check-ziti-face.mjs --self-test   # fixture round-trip: 50/50 green
```

#### 2. `ziti-rust` crate baseline (no network needed)

The CI workflow pins `dtolnay/rust-toolchain@stable`; the local default toolchain can be older, so measure with the CI-era toolchain explicitly:

```powershell
cargo +1.99.0 clippy --workspace --all-targets -- -D warnings
cargo +1.99.0 test  --workspace --lib
cargo +1.99.0 test  --workspace --doc   # doc-comment indented blocks DO compile as Rust; illustrative frame layouts must be fenced as plain "text"
```

#### 3. The 5 live tests (real OpenZiti network required)

Prerequisites:

- a quickstart router up and reachable — the identity files embed the controller/edge URLs, so there is no extra address config;
- an identity pair issued for the dark service `shadow-brain` (host can bind, client can dial), provisioned by `shadow/scripts/openziti-provision.ps1` into `shadow/.ziti/`; the absolute paths are collected in `shadow/.ziti/ziti.env`.

Sanity-check first — the doctor prints each step's real result (status code / full error chain) so you never guess:

```powershell
$env:ZITI_IDENTITY = "C:\Projects\iBLOC\shadow\.ziti\shadow-brain-client.identity.json"
cargo run --offline --manifest-path C:\Projects\iBLOC\synapse\Cargo.toml --features ziti --example ziti_doctor
```

Then two terminals — the host keeps running while the tests dial into it:

```powershell
# -- Terminal 1: self-host the brain (the production serve entry; keep it in the foreground)
$env:ZITI_HOST_IDENTITY = "C:\Projects\iBLOC\shadow\.ziti\shadow-brain-host.identity.json"
$env:ZITI_SERVICE       = "shadow-brain"
cargo run --offline --manifest-path C:\Projects\iBLOC\synapse\Cargo.toml --features ziti --example ziti_v2_brain
```

```powershell
# -- Terminal 2: the 5 #[ignore] live tests
$env:ZITI_IDENTITY          = "C:\Projects\iBLOC\shadow\.ziti\shadow-brain-client.identity.json"
$env:ZITI_SERVICE           = "shadow-brain"
$env:ZITI_TEST_CONVERSATION = "e2e"   # any id works against the example brain; without it the SSE test skips
cargo test --offline --manifest-path C:\Projects\iBLOC\synapse\Cargo.toml --features ziti --lib -- --ignored --nocapture
```

Environment variables (shapes read from source, not from memory):

| Variable | Read by | Meaning | Missing implies |
|------|------|------|------|
| `ZITI_IDENTITY` | live tests (client side) | dial-side identity | tests **skip**, not fail |
| `ZITI_SERVICE` | live tests + host example | dark service name | falls back to `shadow-brain` |
| `ZITI_HOST_IDENTITY` | `ziti_v2_brain` example | bind-side identity | example aborts |
| `ZITI_TEST_CONVERSATION` | SSE test only | conversation id to stream | SSE test skips |
| `ZITI_SHADOW_KEY` / `ZITI_ADMIN_TOKEN` | live tests (door headers) | credentials against a *real* brain (the example brain needs none) | anonymous door headers |

2026-11-06 reading over the quickstart router: **5 passed / 0 failed in 6.48s** (HTTP 200 + correct body / WS 101 + echo / SSE 200 + separate frames / local proxy 200 / full overlay one-command green).

## Migration posture

The module is insulated from callers by the `ZitiGateway` / `ZitiListener` / `ZitiStream` API. Swapping to a different wire implementation (including an eventual official Rust binding) is a local change inside the `ziti-v2` crate plus the `gateway.rs` / `listener.rs` shims; downstream code is unaffected.

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
- [`src/transport/ziti/live_tests.rs`](../../src/transport/ziti/live_tests.rs) — 5 `#[ignore]`d live end-to-end tests (real network)
- [`src/transport/ziti/http_client.rs`](../../src/transport/ziti/http_client.rs) — `ZitiHttpClient` impl
- [`src/transport/mod.rs`](../../src/transport/mod.rs) — `HttpClient` trait (parent of all transports)
- [OpenZiti][openziti] — official project site
- [`docs/concepts/a2a.md`](a2a.md) — A2A protocol reference
