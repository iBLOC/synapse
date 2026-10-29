//! A2A (Agent-to-Agent) protocol integration for Synapse.
//!
//! # v0.2 (recommended)
//!
//! All new code should use the official `a2a-rs` SDK via the wrappers
//! in this module:
//! - [`server::A2AServerBuilder`] hosts a Synapse agent as an A2A v1.0 peer
//! - [`client::A2AClient`] calls a remote peer and returns a Synapse
//!   `AgentStreamEvent` stream
//! - [`card::AgentCard`] is re-exported at the top level for convenience
//! - [`a2a::A2AError`] is re-exported from the SDK at the top level
//!
//! # v0.1 compatibility (deprecated)
//!
//! The v0.1 hand-rolled types `A2AClient`, `A2AServer`, `TaskHandler`,
//! `A2AError`, `A2AResult` are re-exported at the bottom of this
//! module, marked `#[deprecated]`. They are kept in place so that
//! downstream callers built against v0.1 continue to compile unchanged.

pub mod card;
pub mod protocol;

/// Structured tool-event channel for the A2A wire — see the module docs
/// for why the official `StreamResponse` enum alone cannot carry a tool
/// call.
#[cfg(not(target_arch = "wasm32"))]
pub mod tool_event;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(not(target_arch = "wasm32"))]
pub mod client;

#[cfg(not(target_arch = "wasm32"))]
pub mod store;

// v0.1 compat: the entire hand-rolled A2A stack was used by downstream
// projects before v0.2. We re-export the v0.1 symbols here, marked
// deprecated, so downstream code continues to compile without immediate
// churn. New code should use v0.2 wrappers above. Listed FIRST so
// `pub use v1_compat::*` doesn't get shadowed by the v0.2 re-exports below.
#[cfg(not(target_arch = "wasm32"))]
#[allow(deprecated)]
mod v1_compat;
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
pub use v1_compat::*;

// v0.2: re-export common types at the top level.
#[cfg(not(target_arch = "wasm32"))]
pub use a2a::A2AError as SdkA2AError;
#[cfg(not(target_arch = "wasm32"))]
pub use card::AgentCard;
