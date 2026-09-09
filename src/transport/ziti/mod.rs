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
//! Centralising it here means every agent (cortex, tiangong-*,
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
//! ziti::serve(router, "/etc/cortex/identity.json", "cortex").await?;
//! # Ok(()) }
//! ```
//
// # Feature gate
//
// The entire module is wrapped in `#[cfg(feature = "ziti")]`. Opening
// the feature pulls in `ziti-sdk` + hyper glue; without the feature
// the module is absent from the build graph entirely.
//
// # SDK caveat — read this before depending on it
//
// **OpenZiti does not publish an official Rust SDK** at the time of
// writing. The `openziti` GitHub org maintains official SDKs for
// Go / C / Swift / Java / Python / C# but **not Rust**.
//
// This module currently wraps the **third-party `ziti-sdk` crate**
// (`jaschadub/ziti-rust`, MIT, version 0.3.x). That crate is the
// only Rust binding to OpenZiti currently published on crates.io
// and it is community-maintained — not by the OpenZiti project.
//
// ## Mitigation strategy
//
// Callers are insulated from SDK churn by the [`ZitiGateway`] /
// [`ZitiListener`] / [`ZitiStream`] API surface — switching to a
// different Rust binding (or to OpenZiti's official C SDK via FFI)
// only requires changes to `gateway.rs` and `listener.rs`. The
// `serve()` / `serve_ziti_stream()` / `ZitiHttpClient` surface
// stays stable.
//
// If/when the OpenZiti project ships an official Rust SDK, the
// upgrade path is:
//  1. Bump the `ziti-sdk` dep in this crate's `Cargo.toml`.
//  2. Re-implement `gateway.rs` + `listener.rs` against the new
//     SDK's identity / context / connection types.
//  3. Re-export `ZitiStream` from the new SDK (or wrap it) so
//     `ZitiHttpClient` and `serve_ziti_stream` keep working
//     unchanged.
//
// ## Why we don't use the C SDK via FFI today
//
// The OpenZiti project does ship an official `ziti-sdk-c` with an
// Apache-2.0 license. Switching to it via FFI was considered and
// rejected for now:
//  - C-FFI in async Rust adds unsafe-boundary complexity
//    (lifetime management, manual Send/Sync impl, blocking-call
//    → async adapter).
//  - The third-party Rust SDK implements the same wire protocol
//    against the same OpenZiti controllers, so wire compatibility
//    is preserved.
//  - The `ZitiGateway` abstraction means a future FFI rewrite is a
//    local change, not an ecosystem-wide migration.
//
// ## Vendor strategy (recommended for production)
//
// Treat the upstream `jaschadub/ziti-rust` crate as **untrusted
// third-party code** even though it's pinned in Cargo.lock:
//  - Fork it under your org's GitHub namespace.
//  - Vendor it via `[patch.crates-io]` in a workspace-level
//    `Cargo.toml` so a vendored copy is always reachable.
//  - Track upstream via GitHub watches; the wire protocol itself
//    is stable per OpenZiti's documented spec, so a frozen vendor
//    fork will continue to work against newer controllers as long
//    as the protocol version is still supported.
//

#[cfg(feature = "ziti")]
mod config;
#[cfg(feature = "ziti")]
mod connection_pool;
#[cfg(feature = "ziti")]
mod dark_service;
#[cfg(feature = "ziti")]
mod gateway;
#[cfg(feature = "ziti")]
mod http_client;
#[cfg(feature = "ziti")]
mod listener;

#[cfg(feature = "ziti")]
pub use config::{ConfigWatcher, ZitiConfig};
#[cfg(feature = "ziti")]
pub use connection_pool::ConnectionPool;
#[cfg(feature = "ziti")]
pub use dark_service::{serve, serve_ziti_stream};
#[cfg(feature = "ziti")]
pub use gateway::ZitiGateway;
#[cfg(feature = "ziti")]
pub use http_client::{HttpResponse, ZitiHttpClient};
#[cfg(feature = "ziti")]
pub use listener::{DarkServiceHandler, DarkServiceListener, ZitiListener};

#[cfg(feature = "ziti")]
pub use ziti_sdk::ZitiStream;

/// Error type for transport::ziti operations. Re-exported so callers
/// don't need to depend on ziti-sdk directly for error matching.
#[cfg(feature = "ziti")]
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

#[cfg(feature = "ziti")]
pub type Result<T> = std::result::Result<T, ZitiError>;