//! MultiAgent patterns: Router/Supervisor coordination.
//!
//! This module provides:
//! - [`AgentTool`]: Bridges `Agent` trait to `InvokableTool` so any agent
//!   can be exposed as a tool that a supervisor LLM can call.
//! - [`RouterSupervisor`]: A supervisor agent that uses an LLM to route
//!   tasks to specialized sub-agents, with multi-turn support.
//!
//! See `docs/superpowers/specs/2026-06-28-multiagent-router-design.md` for design.

pub mod agent_tool;
pub mod router;

pub use agent_tool::AgentTool;
pub use router::{RouterSupervisor, RouterSupervisorBuilder};
