//! MultiAgent patterns: Router/Supervisor coordination.
//!
//! This module provides:
//! - [`AgentTool`]: Bridges `Agent` trait to `InvokableTool` so any agent
//!   can be exposed as a tool that a supervisor LLM can call.
//! - [`RouterSupervisor`]: A supervisor agent that uses an LLM to route
//!   tasks to specialized sub-agents, with multi-turn support.
//! - [`OrchestratorAgent`]: Decomposes tasks and dispatches sub-agents
//!   in parallel, then synthesizes results.
//!
//! See `docs/superpowers/specs/` for designs.

pub mod agent_tool;
pub mod orchestrator;
pub mod router;

pub use agent_tool::AgentTool;
pub use orchestrator::{OrchestratorAgent, OrchestratorAgentBuilder};
pub use router::{RouterSupervisor, RouterSupervisorBuilder};
