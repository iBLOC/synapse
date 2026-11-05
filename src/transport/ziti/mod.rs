//! Zero-trust transport over OpenZiti.
//!
//! This module is the canonical home for OpenZiti integration in the
//! Shadow platform. It exposes:
//! - [`ZitiGateway`] — load an identity JWT, dial / listen on services
//! - [`ZitiListener`] — accept incoming ZitiStreams on a dark service
//! - [`serve`] — serve an axum `Router` as a dark service (one call)
//! - [`ZitiHttpClient`] — make outbound HTTP requests through Ziti
//! - [`ConnectionPool`] / [`ZitiConfig`] / [`ConfigWatcher`] — auxiliary
//!
//! The module is gated behind the `ziti` Cargo feature so callers
//! that don't need OpenZiti pay no compile-time cost. Enable it in
//! your crate with:
//!
//! ```toml
//! synapse = { path = "../synapse", features = ["ziti"] }
//! ```
//!
//! # Why a transport module?
//!
//! Zero-trust networking is a **platform-level security primitive**.
//! Centralising it here means every agent (encephalon, tiangong-*,
//! third-party) uses the same wire, identity model, and dark-service
//! lifecycle — instead of each caller maintaining its own `XxxDarkService`
//! wrapper.
//!
//! # Quick start (most callers)
//!
//! ```no_run
//! use synapse::transport::ziti;
//!
//! # async fn run(router: axum::Router) -> anyhow::Result<()> {
//! ziti::serve(router, "/etc/encephalon/identity.json", "encephalon").await?;
//! # Ok(()) }
//! ```
//
// # Feature gate
//
// The entire module is wrapped in `#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]`. Opening
// the feature pulls in the v2 wire layer (rustls + hyper glue); without
// the feature the module is absent from the build graph entirely.
//
// # v2 —— 线路层是**自建**的
//
// **OpenZiti does not publish an official Rust SDK.** The `openziti`
// GitHub org maintains official SDKs for Go / C / Swift / Java / Python /
// C# but **not Rust**.
//
// The community crate `ziti-sdk` (jaschadub/ziti-rust, MIT, 0.3.x) speaks
// the **v1** wire protocol — WebSocket + a home-made ZitiMessage frame with
// **no magic**. Against an OpenZiti **v2** controller it simply does not
// work: the router answers the WebSocket upgrade with
// `httparse error: invalid HTTP version`, and the control-plane calls the
// SDK makes (`GET /services/{id}/terminators`,
// `current-identity/edge-routers`) either 404 or come back without the
// fields the SDK needs to deserialise.
//
// ⇒ ⇒ So the wire layer here is **ours** (`v2/`): frame codec, controller
// API, channel, stream, context. Nothing in this module depends on
// `ziti-sdk` any more.
//
// ## API stability —— callers 不受影响
//
// The [`ZitiGateway`] / [`ZitiListener`] / [`ZitiStream`] surface is
// unchanged: the same `dial` / `listen` / `accept` calls, the same
// `ZitiStream` that behaves like a TCP stream. `serve()` /
// `serve_ziti_stream()` / `ZitiHttpClient` / `ConnectionPool` are untouched.
//
// ## 为什么不用 C SDK 的 FFI
//
// The OpenZiti project does ship an official `ziti-sdk-c` (Apache-2.0).
// FFI in async Rust adds unsafe-boundary complexity (lifetime management,
// manual Send/Sync impls, blocking-call → async adapter) — and it would
// still be a second implementation of the same protocol. The self-built v2
// layer is smaller than the FFI shim would be, and it is the only one that
// can talk to a v2 controller without a rewrite.
//
// ## 我们钉在哪些规格上
//
// The wire format is **not** stable across OpenZiti majors — v1 and v2 are
// different protocols. The constants in `v2/proto.rs` are transcribed from
// the official sources (`openziti/sdk-golang` pb + `openziti/channel`
// message.go), and the end-to-end behaviour is pinned by the live tests in
// `live_tests.rs`.
//

#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod config;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod connection_pool;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod dark_service;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod gateway;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod http_client;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod listener;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod proxy;
#[cfg(all(test, feature = "ziti", not(target_arch = "wasm32")))]
mod live_tests;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
mod ws_client;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub mod v2;

#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use config::{ConfigWatcher, ZitiConfig};
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use connection_pool::ConnectionPool;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use dark_service::{serve, serve_ziti_stream};
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use gateway::ZitiGateway;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use http_client::{
    request_over_stream, request_over_stream_bytes, HttpResponse, HttpResponseHead, ZitiHttpClient,
};
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use listener::{DarkServiceHandler, DarkServiceListener, ZitiListener};
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use proxy::{
    proxy_router, spawn_proxy, tcp_connector, ziti_connector, Connector, ProxyState, Upstream,
};
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use ws_client::{connect_ws_over_stream, ZitiWebSocket, ZitiWebSocketClient};

#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use v2::context::Context;
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub use v2::stream::ZitiStream;

/// Error type for transport::ziti operations. Re-exported so callers
/// don't need to depend on ziti-sdk directly for error matching.
#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
#[derive(Debug, thiserror::Error)]
pub enum ZitiError {
    #[error("Failed to load identity: {0}")]
    IdentityLoadError(String),

    #[error("Failed to connect to service: {0}")]
    ConnectionError(String),

    #[error("Failed to listen on service: {0}")]
    ListenError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[cfg(all(feature = "ziti", not(target_arch = "wasm32")))]
pub type Result<T> = std::result::Result<T, ZitiError>;