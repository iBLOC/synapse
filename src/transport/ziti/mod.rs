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