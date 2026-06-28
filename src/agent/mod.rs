//! Multi-agent runtime primitives
//!
//! 参考 Vercel Eve 的 filesystem-first 设计 + 多智能体编排。

pub mod spec;
pub mod loader;
pub mod orchestrator;

pub use spec::{AgentSpec, AgentSpecBuilder, ModelSpec, ToolSpec, ToolKind, SkillSpec, ScheduleSpec, WasmSpec};
pub use loader::AgentLoader;
pub use orchestrator::{AgentOrchestrator, AgentHandle, AgentStatus};