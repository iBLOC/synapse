//! The `StatelessSkill` trait: every lobe skill implements this.
//!
//! A skill is a single async function that takes a typed `Context`
//! and returns a `SkillOutput`. No state lives in the skill itself
//! between calls — all conversation state flows in via `Context` and
//! out via `SkillOutput.updated_history_summary` (caller persists).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[async_trait]
pub trait StatelessSkill: Send + Sync {
    async fn handle(&self, ctx: Box<dyn SkillContext>) -> Result<SkillOutput, String>;
    fn skill_id(&self) -> &'static str;
}

pub trait SkillContext: Send + Sync {
    fn user_id(&self) -> &str;
    fn history_summary(&self) -> Option<&str> {
        None
    }
    fn system_prompt(&self) -> Option<&str> {
        None
    }
    fn extra(&self) -> Option<&serde_json::Value> {
        None
    }
}

/// Helper: every skill needs to parse a JSON context. The `JsonCtx` struct
/// gives a default `user_id` and `history_summary` for free.
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonCtx {
    pub user_id: String,
    #[serde(default)]
    pub history_summary: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl SkillContext for JsonCtx {
    fn user_id(&self) -> &str { &self.user_id }
    fn history_summary(&self) -> Option<&str> { self.history_summary.as_deref() }
    fn system_prompt(&self) -> Option<&str> { self.system_prompt.as_deref() }
    fn extra(&self) -> Option<&serde_json::Value> { Some(&self.extra) }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillOutput {
    pub updated_history_summary: Option<String>,
    pub payload: Value,
    pub audio_url: Option<String>,
}

impl SkillOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            payload: serde_json::json!({ "text": text.into() }),
            ..Default::default()
        }
    }
}

/// Maps skill_id → Arc<dyn StatelessSkill> for A2A dispatch.
pub struct StatelessSkillRouter {
    skills: HashMap<&'static str, Arc<dyn StatelessSkill>>,
}

impl Default for StatelessSkillRouter {
    fn default() -> Self { Self::new() }
}

impl StatelessSkillRouter {
    pub fn new() -> Self {
        Self { skills: HashMap::new() }
    }

    pub fn register(&mut self, skill: Arc<dyn StatelessSkill>) {
        self.skills.insert(skill.skill_id(), skill);
    }

    pub fn list_skills(&self) -> Vec<&'static str> {
        self.skills.keys().copied().collect()
    }

    pub async fn dispatch(
        &self,
        skill_id: &str,
        ctx_json: Value,
    ) -> Result<SkillOutput, String> {
        let skill = self.skills.get(skill_id)
            .ok_or_else(|| format!("unknown skill: {skill_id}"))?;
        let ctx: Box<dyn SkillContext> = Box::new(serde_json::from_value::<JsonCtx>(ctx_json)
            .map_err(|e| format!("invalid context: {e}"))?);
        skill.handle(ctx).await
    }
}
