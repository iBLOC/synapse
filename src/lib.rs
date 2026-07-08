//! Synapse: shared foundation library for the Shadow platform.
//!
//! ## Glob re-exports
//!
//! This crate re-exports many modules' public items at the top level
//! for convenience (`synapse::Agent` instead of
//! `synapse::adk::agent::Agent`). Several module boundaries share
//! common type names (`Message`, `Content`, `TaskStatus`, `MessageDelta`,
//! `MessageDeltaChoice`, etc.) across a2a / adk / agent / components /
//! protocol / scheduler / etc. The `pub use mod::*` re-exports
//! therefore produce a "ambiguous glob re-exports" diagnostic at every
//! pub use site. The conflict is not a soundness bug: every name is
//! still reachable via its fully-qualified path (`synapse::a2a::Message`,
//! `synapse::adk::Message`, `synapse::protocol::adapter::TaskStatus`,
//! etc.); the warning only flags that the *glob* form is ambiguous.
//! Downstream consumers should prefer the fully-qualified form
//! for types that collide across modules; the preludes (`adk::prelude`,
//! `a2a::prelude`) re-export the most-used names per-module.

#![allow(ambiguous_glob_reexports)]

pub mod a2a;
pub mod adk;
pub mod agent;
pub mod callbacks;
pub mod channels;
pub mod coding_tools;
pub mod components;
pub mod compose;
pub mod llm;
pub mod lobe;
pub mod mcp;
pub mod protocol;
pub mod providers;
pub mod scheduler;
pub mod schema;
pub mod session;
pub mod skills;
pub mod stream;
pub mod task;
pub mod thought;
pub mod transport;
pub mod utils;
pub mod wasm;

pub use a2a::*;
pub use adk::*;
pub use agent::*;
pub use callbacks::*;
pub use channels::*;
pub use components::*;
pub use compose::*;
pub use protocol::*;
pub use providers::*;
pub use scheduler::*;
pub use schema::*;
pub use session::*;
pub use skills::*;
pub use task::*;
pub use utils::*;
pub use wasm::*;
