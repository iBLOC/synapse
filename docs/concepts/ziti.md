---
title: transport::ziti
---

# `transport::ziti` · OpenZiti 零信任传输

Zero-trust A2A over [OpenZiti][openziti]. Public IPs are never exposed — every service is a *dark service* accessible only via an OpenZiti identity that the controller has authorised.
*通过 [OpenZiti][openziti] 实现零信任 A2A。公网 IP 完全不暴露 —— 每个服务都是 *dark service*，只有 OpenZiti controller 授权过的 identity 才能访问。*

The module is gated behind the `ziti` Cargo feature so callers that don't need OpenZiti pay no compile-time cost.
*本模块由 Cargo feature `ziti` 门控，不需要 OpenZiti 的调用方完全不承担编译期开销。*

```toml
synapse = { path = "../synapse", features = ["ziti"] }
```

[openziti]: https://openziti.io/

## Why a transport module · 为什么是 transport 模块

Zero-trust networking is a **platform-level security primitive**. Centralising it here means every agent (encephalon, tiangong-*, third-party) uses the same wire, identity model, and dark-service lifecycle — instead of each caller maintaining its own `XxxDarkService` wrapper.
*零信任网络是**平台级安全原语**。集中在这里意味着每个 agent（encephalon、tiangong-*、第三方）用同一条 wire、同一个 identity 模型、同一种 dark-service 生命周期 —— 而不是每个调用方自己维护一个 `XxxDarkService`。*

It is the canonical transport for A2A when the public network is untrusted (multi-cloud, edge gateways, on-prem ↔ cloud). Use it **instead of** mTLS-over-LAN, not in addition.
*A2A 在公网不可信时（多云、边缘网关、on-prem ↔ cloud）的标准传输。**替代** mTLS-over-LAN，而不是叠加使用。*

## Public surface · 公开 API

| Item / 项目 | Type / 类型 | Purpose / 用途 |
|------|------|------|
| `ZitiGateway` | `struct` | Load an OpenZiti identity JWT, get a context / 加载 OpenZiti identity JWT，得到 context |
| `ZitiGateway::dial(service_name)` | `async fn -> Result<ZitiStream, ZitiError>` | Peer connection to a service / 连接到某个 service |
| `ZitiGateway::listen(service_name)` | `async fn -> Result<ZitiListener, ZitiError>` | Bind a dark service / 绑定一个 dark service |
| `ZitiListener` | `struct` | Accept incoming `ZitiStream`s on a dark service / 在 dark service 上接受 incoming `ZitiStream` |
| `ZitiListener::accept()` | `async fn -> Result<ZitiStream, ZitiError>` | Pull one incoming connection / 拉一条 incoming 连接 |
| `serve(router, identity_path, service_name)` | `async fn` | **One-call convenience** — load identity + register dark service + accept loop + serve each accepted `ZitiStream` as an axum `Router` (hyper over Ziti) / 一行搞定：加载 identity + 注册 dark service + accept 循环 + 把每条 stream 挂上 axum `Router` |
| `serve_ziti_stream(stream, router)` | `async fn` | Serve one axum `Router` on one accepted `ZitiStream` (in-process glue: hyper over Ziti) / 在一条 accepted `ZitiStream` 上跑 axum `Router`（进程内 glue：hyper over Ziti） |
| `ZitiHttpClient` | `struct` | Outbound HTTP through Ziti / 通过 Ziti 出站 HTTP |
| `ConnectionPool` | `struct` | Reusable Ziti streams / 可复用的 Ziti stream |
| `ZitiConfig` | `struct` | JSON config (loaded from controller-side identity file) / JSON 配置 |
| `ConfigWatcher` | `struct` | Watch config file changes / 监听配置变更 |
| `ZitiStream` | re-export from `ziti-sdk` | One bidirectional Ziti connection / 一条双向 Ziti 连接 |
| `ZitiError` | `enum` | Identity / connection / listen / IO errors / 各种 Ziti 错误 |

## Quick start (most callers) · 快速开始（绝大多数调用方）

Replace the four-line `TcpListener` dance with a single call:
*把四行 `TcpListener` 模板换成一次调用：*

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
*Identity 文件是从 OpenZiti controller 导出的 JSON 文档。Service 名必须在 controller 上预先配好 policy。*

## Outbound HTTP through Ziti · 通过 Ziti 出站 HTTP

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
*`ZitiHttpClient` 与 `ReqwestHttpClient`、`WasiHttpClient` 同源 `HttpClient` trait，上层（A2A client、MCP client、OpenAI provider）不关心底下走哪条 transport。*

## Feature gate · Feature 开关

The entire module is wrapped in `#[cfg(feature = "ziti")]`. Opening the feature pulls in `ziti-sdk` + hyper glue; without the feature the module is absent from the build graph entirely.
*整个模块都被 `#[cfg(feature = "ziti")]` 包裹。开 feature 才引入 `ziti-sdk` + hyper glue；不开 feature 模块从 build graph 里完全消失。*

```bash
cargo build  -p synapse --features ziti
cargo test   -p synapse --features ziti --lib transport::ziti
```

## SDK caveat — read this before depending on it · SDK 注意事项（依赖前必读）

⚠️ **OpenZiti does not publish an official Rust SDK**. The `openziti` GitHub org maintains official SDKs for Go / C / Swift / Java / Python / C# but **not Rust**.
*⚠️ **OpenZiti 没有官方 Rust SDK**。`openziti` GitHub org 维护 Go / C / Swift / Java / Python / C# 的官方 SDK，但**没有 Rust**。*

This module currently wraps the **third-party `ziti-sdk` crate** (`jaschadub/ziti-rust`, MIT, version 0.3.x). That crate is the only Rust binding to OpenZiti currently published on crates.io and it is **community-maintained** — not by the OpenZiti project.
*本模块目前封装的是**第三方 `ziti-sdk` crate**（`jaschadub/ziti-rust`，MIT，0.3.x）。这是 crates.io 上唯一的 Rust 绑定，**社区维护**，不是 OpenZiti 项目本身。*

### Mitigation strategy · 缓解策略

Callers are insulated from SDK churn by the `ZitiGateway` / `ZitiListener` / `ZitiStream` API surface — switching to a different Rust binding (or to OpenZiti's official C SDK via FFI) only requires changes to `gateway.rs` and `listener.rs`. The `serve()` / `serve_ziti_stream()` / `ZitiHttpClient` surface stays stable.
*调用方通过 `ZitiGateway` / `ZitiListener` / `ZitiStream` API 表面与 SDK 解耦 —— 换到其他 Rust 绑定（或者走 OpenZiti 官方 C SDK 的 FFI）只需要改 `gateway.rs` 和 `listener.rs`。`serve()` / `serve_ziti_stream()` / `ZitiHttpClient` 表面保持稳定。*

If/when the OpenZiti project ships an official Rust SDK, the upgrade path is:
*等 OpenZiti 出官方 Rust SDK 后，升级路径是：*

1. Bump the `ziti-sdk` dep in this crate's `Cargo.toml` / 在 `Cargo.toml` 升 `ziti-sdk` 依赖
2. Re-implement `gateway.rs` + `listener.rs` against the new SDK's identity / context / connection types / 在新的 SDK 上重写 `gateway.rs` + `listener.rs`
3. Re-export `ZitiStream` from the new SDK (or wrap it) so `ZitiHttpClient` and `serve_ziti_stream` keep working unchanged / 用新 SDK 重导出 `ZitiStream`（或包一层）让 `ZitiHttpClient` 与 `serve_ziti_stream` 不变

### Why we don't use the C SDK via FFI today · 为什么不直接走 C SDK + FFI

The OpenZiti project does ship an official `ziti-sdk-c` with an Apache-2.0 license. Switching to it via FFI was considered and rejected for now:
*OpenZiti 项目确实有官方 `ziti-sdk-c`，Apache-2.0。也评估过 FFI 方案，目前未采纳：*

- C-FFI in async Rust adds unsafe-boundary complexity (lifetime management, manual `Send`/`Sync` impl, blocking-call → async adapter).
  *async Rust 中搞 C-FFI 会带来 unsafe 边界复杂度（生命周期管理、手动 `Send`/`Sync` impl、阻塞调用到 async 适配器）。*
- The third-party Rust SDK implements the same wire protocol against the same OpenZiti controllers, so wire compatibility is preserved.
  *第三方 Rust SDK 实现的是同一条 wire protocol、跑同一个 OpenZiti controller，wire 兼容性保留。*
- The `ZitiGateway` abstraction means a future FFI rewrite is a local change, not an ecosystem-wide migration.
  *有了 `ZitiGateway` 抽象，将来改 FFI 是局部改动，不需要全生态迁移。*

### Vendor strategy (recommended for production) · Vendor 策略（生产建议）

Treat the upstream `jaschadub/ziti-rust` crate as **untrusted third-party code** even though it's pinned in `Cargo.lock`:
*把上游 `jaschadub/ziti-rust` 当作**不受信任的第三方代码**处理，即使已经在 `Cargo.lock` 里 pin 死：*

- Fork it under your org's GitHub namespace / fork 到你组织的 GitHub namespace 下
- Vendor it via `[patch.crates-io]` in a workspace-level `Cargo.toml` so a vendored copy is always reachable / 在 workspace 顶层 `Cargo.toml` 用 `[patch.crates-io]` vendor 一份
- Track upstream via GitHub watches; the wire protocol itself is stable per OpenZiti's documented spec, so a frozen vendor fork will continue to work against newer controllers as long as the protocol version is still supported / 用 GitHub watch 跟踪上游；wire protocol 本身按 OpenZiti 公开规范是稳定的，frozen vendor fork 只要 protocol version 还在 controller 支持范围内就能继续用

## Migration posture · 升级路径

The module is insulated from callers by the `ZitiGateway` / `ZitiListener` / `ZitiStream` API. Swapping to a different SDK (including an eventual official Rust binding) is a local change to `gateway.rs` + `listener.rs`; downstream code is unaffected.
*本模块通过 `ZitiGateway` / `ZitiListener` / `ZitiStream` 与调用方解耦。换 SDK（包括将来的官方 Rust 绑定）只需要局部改 `gateway.rs` + `listener.rs`；下游代码不受影响。*

## When to use `transport::ziti` · 何时使用 `transport::ziti`

| Scenario / 场景 | Use ziti? / 用 ziti? |
|------|:---:|
| A2A across untrusted networks (multi-cloud, edge ↔ cloud) / 跨不可信网络的 A2A（多云、边缘 ↔ 云） | ✅ |
| Service with strict zero-trust posture / 服务本身要求零信任 | ✅ |
| LAN-only deployment with mTLS already in place / 纯 LAN 部署且已经用 mTLS | ❌ mTLS-over-TCP is simpler / 直接 mTLS-over-TCP 更简单 |
| Public-facing service that genuinely needs a public IP / 公网 IP 必须暴露给第三方的服务 | ❌ ziti's whole point is hiding the IP / ziti 整体定位就是藏 IP |
| WASM sandboxed lobe calling other agents / WASM 沙箱 lobe 调其他 agent | ❌ use `WasiHttpClient` / 用 `WasiHttpClient` |
| Browser-facing traffic / 浏览器直接访问的流量 | ❌ ziti is not a replacement for TLS+CDN / ziti 不能替代 TLS+CDN |

## See also · 相关阅读

- [`src/transport/ziti/mod.rs`](../../src/transport/ziti/mod.rs) — module-level rustdoc (full SDK caveat + vendor strategy)
- [`src/transport/ziti/gateway.rs`](../../src/transport/ziti/gateway.rs) — `ZitiGateway` impl
- [`src/transport/ziti/listener.rs`](../../src/transport/ziti/listener.rs) — `ZitiListener` impl
- [`src/transport/ziti/dark_service.rs`](../../src/transport/ziti/dark_service.rs) — `serve()` + `serve_ziti_stream()`
- [`src/transport/ziti/http_client.rs`](../../src/transport/ziti/http_client.rs) — `ZitiHttpClient` impl
- [`src/transport/mod.rs`](../../src/transport/mod.rs) — `HttpClient` trait (parent of all transports)
- [OpenZiti][openziti] — official project site
- [`docs/concepts/a2a.md`](a2a.md) — A2A protocol reference
