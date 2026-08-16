pub mod namespace;

pub use namespace::{NamespaceError, ScopedSkillRouter};

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub type SkillResult<T> = Result<T, SkillError>;

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("execution error: {0}")]
    Execution(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("skill disabled")]
    Disabled,
}

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub messages: Vec<crate::schema::Message>,
    pub state: HashMap<String, serde_json::Value>,
}

impl SkillContext {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            state: HashMap::new(),
        }
    }

    pub fn with_messages(mut self, messages: Vec<crate::schema::Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_state(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.state.insert(key.into(), value);
        self
    }
}

impl Default for SkillContext {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn keywords(&self) -> Vec<String>;
    fn patterns(&self) -> Vec<String>;
    fn max_context_tokens(&self) -> Option<usize>;
    async fn execute(&self, ctx: &SkillContext, input: &str) -> SkillResult<String>;
}

pub struct SkillManager {
    skills: HashMap<String, Arc<dyn Skill>>,
    enabled: bool,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            enabled: true,
        }
    }

    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        self.skills.insert(skill.name().to_string(), skill);
    }

    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Skill>> {
        self.skills.get(name)
    }

    pub fn find_skill(&self, input: &str) -> Option<&Arc<dyn Skill>> {
        let input_lower = input.to_lowercase();

        for skill in self.skills.values() {
            for keyword in skill.keywords() {
                if input_lower.contains(&keyword.to_lowercase()) {
                    return Some(skill);
                }
            }

            for pattern in skill.patterns() {
                if regex::Regex::new(&pattern)
                    .map(|re| re.is_match(input))
                    .unwrap_or(false)
                {
                    return Some(skill);
                }
            }
        }

        None
    }

    pub fn list_skills(&self) -> Vec<SkillInfo> {
        self.skills
            .values()
            .map(|s| SkillInfo {
                name: s.name(),
                description: s.description(),
                keywords: s.keywords(),
                patterns: s.patterns(),
            })
            .collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        ctx: &SkillContext,
        input: &str,
    ) -> SkillResult<String> {
        if !self.enabled {
            return Err(SkillError::Disabled);
        }

        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| SkillError::NotFound(name.to_string()))?;

        skill.execute(ctx, input).await
    }

    pub async fn execute_matched(&self, ctx: &SkillContext, input: &str) -> SkillResult<String> {
        if !self.enabled {
            return Err(SkillError::Disabled);
        }

        let skill = self
            .find_skill(input)
            .ok_or_else(|| SkillError::NotFound("No matching skill".to_string()))?;

        skill.execute(ctx, input).await
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub patterns: Vec<String>,
}

pub struct SimpleSkill {
    name: String,
    description: String,
    keywords: Vec<String>,
    patterns: Vec<String>,
    max_context_tokens: Option<usize>,
    handler: Arc<dyn Fn(&SkillContext, &str) -> Result<String, String> + Send + Sync>,
}

impl SimpleSkill {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        handler: impl Fn(&SkillContext, &str) -> Result<String, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            keywords: Vec::new(),
            patterns: Vec::new(),
            max_context_tokens: None,
            handler: Arc::new(handler),
        }
    }

    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    pub fn patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    pub fn max_context_tokens(mut self, tokens: usize) -> Self {
        self.max_context_tokens = Some(tokens);
        self
    }
}

#[async_trait]
impl Skill for SimpleSkill {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn keywords(&self) -> Vec<String> {
        self.keywords.clone()
    }

    fn patterns(&self) -> Vec<String> {
        self.patterns.clone()
    }

    fn max_context_tokens(&self) -> Option<usize> {
        self.max_context_tokens
    }

    async fn execute(&self, ctx: &SkillContext, input: &str) -> SkillResult<String> {
        (self.handler)(ctx, input).map_err(|e| SkillError::Execution(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skill_manager_register_and_get() {
        let mut manager = SkillManager::new();

        let skill = Arc::new(
            SimpleSkill::new("test_skill", "A test skill", |_ctx, _input| {
                Ok("executed".to_string())
            })
            .keywords(vec!["test".to_string()]),
        );

        manager.register(skill);

        let retrieved = manager.get("test_skill");
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_skill_manager_find_by_keyword() {
        let mut manager = SkillManager::new();

        let skill = Arc::new(
            SimpleSkill::new("calculator", "Performs calculations", |_ctx, _input| {
                Ok("calculated".to_string())
            })
            .keywords(vec!["calculate".to_string(), "compute".to_string()]),
        );

        manager.register(skill);

        let found = manager.find_skill("please calculate 2+2");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "calculator");
    }

    #[tokio::test]
    async fn test_skill_manager_execute_matched() {
        let mut manager = SkillManager::new();

        let skill = Arc::new(
            SimpleSkill::new("greet", "Greets the user", |_ctx, _input| {
                Ok("Hello, World!".to_string())
            })
            .keywords(vec!["hello".to_string(), "hi".to_string()]),
        );

        manager.register(skill);

        let ctx = SkillContext::new();
        let result = manager.execute_matched(&ctx, "say hello").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_skill_manager_execute_not_found() {
        let manager = SkillManager::new();

        let ctx = SkillContext::new();
        let result = manager.execute_matched(&ctx, "some unknown input").await;

        assert!(result.is_err());
    }
}
