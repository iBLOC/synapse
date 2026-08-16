use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

use crate::lobe::{SkillContext, SkillOutput, StatelessSkill};

#[derive(Debug, Error)]
pub enum NamespaceError {
    #[error("namespace not found: {0}")]
    NamespaceNotFound(String),
    #[error("skill not found: {0}")]
    SkillNotFound(String),
    #[error("invalid name format: {0}")]
    InvalidName(String),
    #[error("namespace already exists: {0}")]
    NamespaceExists(String),
    #[error("skill error: {0}")]
    Skill(String),
}

pub struct Namespace {
    name: String,
    skills: HashMap<String, Arc<dyn StatelessSkill>>,
}

impl Namespace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Arc<dyn StatelessSkill>) {
        self.skills.insert(skill.skill_id().to_string(), skill);
    }

    pub fn get(&self, skill_name: &str) -> Option<&Arc<dyn StatelessSkill>> {
        self.skills.get(skill_name)
    }

    pub fn list_skills(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    pub async fn dispatch(&self, skill_name: &str, ctx: Value) -> Result<SkillOutput, NamespaceError> {
        let skill = self.skills.get(skill_name)
            .ok_or_else(|| NamespaceError::SkillNotFound(skill_name.to_string()))?;

        let skill_ctx = Box::new(serde_json::from_value::<crate::lobe::JsonCtx>(ctx)
            .map_err(|e| NamespaceError::Skill(e.to_string()))?);

        skill.handle(skill_ctx).await
            .map_err(|e| NamespaceError::Skill(e))
    }
}

pub struct ScopedSkillRouter {
    namespaces: HashMap<String, Namespace>,
}

impl ScopedSkillRouter {
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::new(),
        }
    }

    pub fn register_namespace(&mut self, name: impl Into<String>) -> Result<(), NamespaceError> {
        let name = name.into();
        if self.namespaces.contains_key(&name) {
            return Err(NamespaceError::NamespaceExists(name));
        }
        self.namespaces.insert(name.clone(), Namespace::new(name));
        Ok(())
    }

    pub fn register_skill(
        &mut self,
        namespace: &str,
        skill: Arc<dyn StatelessSkill>,
    ) -> Result<(), NamespaceError> {
        let ns = self.namespaces.get_mut(namespace)
            .ok_or_else(|| NamespaceError::NamespaceNotFound(namespace.to_string()))?;
        ns.register(skill);
        Ok(())
    }

    pub fn get_skill(&self, qualified_name: &str) -> Option<&Arc<dyn StatelessSkill>> {
        let (namespace, skill_name) = match parse_qualified_name(qualified_name) {
            Ok((ns, name)) => (ns, name),
            Err(_) => return None,
        };

        self.namespaces.get(namespace)?.get(skill_name)
    }

    pub async fn dispatch(&self, qualified_name: &str, ctx: Value) -> Result<SkillOutput, NamespaceError> {
        let (namespace, skill_name) = parse_qualified_name(qualified_name)?;

        let ns = self.namespaces.get(namespace)
            .ok_or_else(|| NamespaceError::NamespaceNotFound(namespace.to_string()))?;

        ns.dispatch(skill_name, ctx).await
    }

    pub fn list_namespaces(&self) -> Vec<String> {
        self.namespaces.keys().cloned().collect()
    }

    pub fn list_skills(&self, namespace: &str) -> Result<Vec<String>, NamespaceError> {
        let ns = self.namespaces.get(namespace)
            .ok_or_else(|| NamespaceError::NamespaceNotFound(namespace.to_string()))?;
        Ok(ns.list_skills())
    }

    pub fn list_all_skills(&self) -> Vec<String> {
        let mut all = Vec::new();
        for (ns_name, ns) in &self.namespaces {
            for skill_name in ns.list_skills() {
                all.push(format!("{}__{}", ns_name, skill_name));
            }
        }
        all
    }
}

impl Default for ScopedSkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_qualified_name(name: &str) -> Result<(&str, &str), NamespaceError> {
    let parts: Vec<&str> = name.splitn(2, "__").collect();
    if parts.len() != 2 {
        return Err(NamespaceError::InvalidName(name.to_string()));
    }
    Ok((parts[0], parts[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoSkill;

    #[async_trait::async_trait]
    impl StatelessSkill for EchoSkill {
        fn skill_id(&self) -> &'static str { "echo" }

        async fn handle(&self, ctx: Box<dyn SkillContext>) -> Result<SkillOutput, String> {
            let extra = ctx.extra().ok_or("missing extra")?;
            let input = extra.get("input").and_then(|v| v.as_str()).unwrap_or("");
            Ok(SkillOutput::text(input.to_string()))
        }
    }

    #[tokio::test]
    async fn test_scoped_router_basic() {
        let mut router = ScopedSkillRouter::new();
        router.register_namespace("mcp").unwrap();
        router.register_skill("mcp", Arc::new(EchoSkill)).unwrap();

        let result = router.dispatch("mcp__echo", serde_json::json!({
            "user_id": "test",
            "input": "hello"
        })).await.unwrap();

        assert_eq!(result.payload, serde_json::json!({"text": "hello"}));
    }

    #[tokio::test]
    async fn test_scoped_router_namespace_not_found() {
        let router = ScopedSkillRouter::new();
        let result = router.dispatch("unknown__echo", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scoped_router_skill_not_found() {
        let mut router = ScopedSkillRouter::new();
        router.register_namespace("mcp").unwrap();

        let result = router.dispatch("mcp__unknown", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scoped_router_invalid_name() {
        let router = ScopedSkillRouter::new();
        let result = router.dispatch("invalid_name", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scoped_router_list_all() {
        let mut router = ScopedSkillRouter::new();
        router.register_namespace("mcp").unwrap();
        router.register_namespace("web").unwrap();
        router.register_skill("mcp", Arc::new(EchoSkill)).unwrap();

        let all = router.list_all_skills();
        assert_eq!(all.len(), 1);
        assert!(all.contains(&"mcp__echo".to_string()));
    }
}
