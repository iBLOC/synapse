//! ACP (Agent Client Protocol) client integration.
//!
//! Drives external coding agents that speak the [Agent Client Protocol][acp]
//! (opencode, claude-code-acp, codex-acp, gemini, …) over stdio and surfaces
//! their output as a normal Synapse [`AgentStreamEvent`] stream.
//!
//! Currently this is a **client-only** integration: Synapse speaks *to* an
//! external ACP agent and consumes its responses. Implementing the ACP
//! `Agent` role inside Synapse is out of scope for this module.
//!
//! # Wire model
//!
//! ```text
//!  ┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
//!  │ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
//!  │ AcpClient      │                        │ (external subprocess)   │
//!  └────────────────┘                        └────────────────────────┘
//! ```
//!
//! The SDK spawns the subprocess for us (via [`AcpAgent`]); we never touch
//! `tokio::process::Command` directly. [`AcpClient`] owns one stdio
//! transport plus the background dispatch task; each call to
//! [`AcpSession::prompt`] opens a fresh `agent-client-protocol` session
//! (`session/new`) on that transport.
//!
//! # Build
//!
//! Gated by the `acp` feature (off by default) and only available on
//! non-wasm targets — the upstream SDK does not expose `AcpAgent` or
//! `Stdio` on `wasm32`.
//!
//! ```bash
//! cargo build -p synapse --features acp
//! ```
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(feature = "acp")]
//! # async fn run() -> Result<(), synapse::acp::AcpError> {
//! use synapse::acp::{AcpAgentSpec, AcpClient};
//!
//! // Talk to `opencode --acp` (or any other ACP agent).
//! let client = AcpClient::connect(AcpAgentSpec::opencode()).await?;
//!
//! // Send one prompt; consume the stream of `AgentStreamEvent`s.
//! let mut events = client.session().prompt("Summarise this file").await?;
//! while let Some(ev) = events.next().await {
//!     // …
//! }
//! # Ok(()) }
//! ```
//!
//! [acp]: https://agentclientprotocol.com/

#[cfg(not(target_arch = "wasm32"))]
mod client;
#[cfg(not(target_arch = "wasm32"))]
mod session;
#[cfg(not(target_arch = "wasm32"))]
mod spec;
#[cfg(not(target_arch = "wasm32"))]
mod translate;

#[cfg(all(not(target_arch = "wasm32"), feature = "acp"))]
pub use client::{AcpClient, AcpSessionHandle};
#[cfg(all(not(target_arch = "wasm32"), feature = "acp"))]
pub use session::AcpSession;
#[cfg(all(not(target_arch = "wasm32"), feature = "acp"))]
pub use spec::AcpAgentSpec;

/// Errors produced by the ACP client.
#[cfg(all(not(target_arch = "wasm32"), feature = "acp"))]
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    /// Underlying ACP SDK failure (transport, protocol, RPC).
    #[error("ACP SDK error: {0}")]
    Sdk(#[from] agent_client_protocol::Error),

    /// The user cancelled the prompt via [`crate::stream::CancelHandle`].
    #[error("ACP prompt cancelled by caller")]
    Cancelled,
}