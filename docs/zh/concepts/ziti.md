---
title: transport::ziti
---

# OpenZiti 零信任传输

通过 [OpenZiti][openziti] 实现零信任 A2A。公网 IP 完全不暴露 —— 每个服务都是 *dark service*，只有 OpenZiti controller 授权过的 identity 才能访问。

本模块由 Cargo feature `ziti` 门控，不需要 OpenZiti 的调用方完全不承担编译期开销。

```toml
synapse = { path = "../synapse", features = ["ziti"] }
```

## 为什么是 transport 模块

零信任网络是**平台级安全原语**。集中在这里意味着每个 agent（encephalon、tiangong-*、第三方）用同一条 wire、同一个 identity 模型、同一种 dark-service 生命周期 —— 而不是每个调用方自己维护一个 `XxxDarkService`。

A2A 在公网不可信时（多云、边缘网关、on-prem ↔ cloud）的标准传输。**替代** mTLS-over-LAN，而不是叠加使用。

## 公开 API

| 项目 | 类型 | 用途 |
|------|------|------|
| `ZitiGateway` | `struct` | 加载 OpenZiti identity JWT，得到 context |
| `ZitiGateway::dial(service_name)` | `async fn -> Result<ZitiStream, ZitiError>` | 连接到某个 service |
| `ZitiGateway::listen(service_name)` | `async fn -> Result<ZitiListener, ZitiError>` | 绑定一个 dark service |
| `ZitiListener` | `struct` | 在 dark service 上接受 incoming `ZitiStream` |
| `ZitiListener::accept()` | `async fn -> Result<ZitiStream, ZitiError>` | 拉一条 incoming 连接 |
| `serve(router, identity_path, service_name)` | `async fn` | 一行搞定：加载 identity + 注册 dark service + accept 循环 + 把每条 stream 挂上 axum `Router` |
| `serve_ziti_stream(stream, router)` | `async fn` | 在一条 accepted `ZitiStream` 上跑 axum `Router`（进程内 glue：hyper over Ziti） |
| `ZitiHttpClient` | `struct` | 通过 Ziti 出站 HTTP |
| `ConnectionPool` | `struct` | 可复用的 Ziti stream |
| `ZitiConfig` | `struct` | JSON 配置 |
| `ConfigWatcher` | `struct` | 监听配置变更 |
| `ZitiStream` | re-export from `ziti-sdk` | 一条双向 Ziti 连接 |
| `ZitiError` | `enum` | 各种 Ziti 错误 |

## 快速开始（绝大多数调用方）

把四行 `TcpListener` 模板换成一次调用：

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

Identity 文件是从 OpenZiti controller 导出的 JSON 文档。Service 名必须在 controller 上预先配好 policy。

## 通过 Ziti 出站 HTTP

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

`ZitiHttpClient` 与 `ReqwestHttpClient`、`WasiHttpClient` 同源 `HttpClient` trait，上层（A2A client、MCP client、OpenAI provider）不关心底下走哪条 transport。

## Feature 开关

整个模块都被 `#[cfg(feature = "ziti")]` 包裹。开 feature 才引入 `ziti-sdk` + hyper glue；不开 feature 模块从 build graph 里完全消失。

```bash
cargo build  -p synapse --features ziti
cargo test   -p synapse --features ziti --lib transport::ziti
```

## SDK 注意事项（依赖前必读）

⚠️ **OpenZiti 没有官方 Rust SDK**。`openziti` GitHub org 维护 Go / C / Swift / Java / Python / C# 的官方 SDK，但**没有 Rust**。

本模块目前封装的是**第三方 `ziti-sdk` crate**（`jaschadub/ziti-rust`，MIT，0.3.x）。这是 crates.io 上唯一的 Rust 绑定，**社区维护**，不是 OpenZiti 项目本身。

### 缓解策略

调用方通过 `ZitiGateway` / `ZitiListener` / `ZitiStream` API 表面与 SDK 解耦 —— 换到其他 Rust 绑定（或者走 OpenZiti 官方 C SDK 的 FFI）只需要改 `gateway.rs` 和 `listener.rs`。`serve()` / `serve_ziti_stream()` / `ZitiHttpClient` 表面保持稳定。

等 OpenZiti 出官方 Rust SDK 后，升级路径是：

1. 在 `Cargo.toml` 升 `ziti-sdk` 依赖
2. 在新的 SDK 上重写 `gateway.rs` + `listener.rs`
3. 用新 SDK 重导出 `ZitiStream`（或包一层）让 `ZitiHttpClient` 与 `serve_ziti_stream` 不变

### 为什么不直接走 C SDK + FFI

OpenZiti 项目确实有官方 `ziti-sdk-c`，Apache-2.0。也评估过 FFI 方案，目前未采纳：

  - async Rust 中搞 C-FFI 会带来 unsafe 边界复杂度（生命周期管理、手动 `Send`/`Sync` impl、阻塞调用到 async 适配器）。
  - 第三方 Rust SDK 实现的是同一条 wire protocol、跑同一个 OpenZiti controller，wire 兼容性保留。
  - 有了 `ZitiGateway` 抽象，将来改 FFI 是局部改动，不需要全生态迁移。

### Vendor 策略（生产建议）

把上游 `jaschadub/ziti-rust` 当作**不受信任的第三方代码**处理，即使已经在 `Cargo.lock` 里 pin 死：

- fork 到你组织的 GitHub namespace 下
- 在 workspace 顶层 `Cargo.toml` 用 `[patch.crates-io]` vendor 一份
- 用 GitHub watch 跟踪上游；wire protocol 本身按 OpenZiti 公开规范是稳定的，frozen vendor fork 只要 protocol version 还在 controller 支持范围内就能继续用

## 升级路径

本模块通过 `ZitiGateway` / `ZitiListener` / `ZitiStream` 与调用方解耦。换 SDK（包括将来的官方 Rust 绑定）只需要局部改 `gateway.rs` + `listener.rs`；下游代码不受影响。

## 何时使用 `transport::ziti`

| 场景 | 用 ziti? |
|------|:---:|
| 跨不可信网络的 A2A（多云、边缘 ↔ 云） | ✅ |
| 服务本身要求零信任 | ✅ |
| 纯 LAN 部署且已经用 mTLS | 直接 mTLS-over-TCP 更简单 |
| 公网 IP 必须暴露给第三方的服务 | ziti 整体定位就是藏 IP |
| WASM 沙箱 lobe 调其他 agent | 用 `WasiHttpClient` |
| 浏览器直接访问的流量 | ziti 不能替代 TLS+CDN |

## 相关阅读
