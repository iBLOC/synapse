//! `synapse::lobe` — thin glue layer for building A2A-speaking agent lobes.
//!
//! Optional module that 4 大脑分区 projects (temporal/parietal/occipital/
//! memory) can use to bootstrap a stateless skill executor with cortex
//! registration, A2A server, history summarization, and env config
//! loading in ~100 lines of `main.rs`.
//!
//! Reference frameworks: LangChain (Python), BeeAI (Python), Eino (Go).
//! `synapse::lobe` re-uses existing `synapse::a2a::*`, `synapse::providers::*`
//! rather than wrapping or replacing them. It's a glue layer, not a
//! framework.

pub use crate::a2a::card::{AgentCard, Authentication, Capabilities, Provider, SkillReference};
pub use crate::a2a::protocol::{Message, MessagePart, MessageRole, Task, TaskState};
pub use crate::providers::LLMProvider;

pub mod stateless_skill;
pub mod history;
pub mod env_config;
pub mod provider_set;
pub mod cortex_registration;
pub mod a2a_server;
pub mod lobe_app;
pub mod provider_traits;

pub use stateless_skill::{
    StatelessSkill, SkillContext, SkillOutput, StatelessSkillRouter, JsonCtx,
};
pub use history::HistorySummarizer;
pub use env_config::LobeEnv;
pub use provider_set::ProviderSet;
pub use cortex_registration::register_with_cortex;
pub use a2a_server::build_a2a_router;
pub use lobe_app::LobeApp;
pub use provider_traits::{EmotionProvider, EmotionInput, EmotionOutput};
