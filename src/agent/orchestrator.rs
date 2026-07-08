use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::spec::AgentSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Running,
    Waiting,
    Completed,
    Failed,
    Isolated,
}

#[derive(Clone)]
pub struct AgentHandle {
    pub id: String,
    pub spec: AgentSpec,
    pub status: Arc<RwLock<AgentStatus>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AgentHandle {
    pub fn new(spec: AgentSpec) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: spec.id.clone(),
            spec,
            status: Arc::new(RwLock::new(AgentStatus::Idle)),
            created_at: now,
        }
    }

    pub fn status(&self) -> AgentStatus {
        *self.status.read()
    }

    pub fn set_status(&self, status: AgentStatus) {
        *self.status.write() = status;
    }
}

impl std::fmt::Debug for AgentHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHandle")
            .field("id", &self.id)
            .field("status", &self.status())
            .field("created_at", &self.created_at)
            .finish()
    }
}

pub struct AgentOrchestrator {
    agents: DashMap<String, AgentHandle>,
    bindings: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            bindings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, spec: AgentSpec) -> Result<AgentHandle, String> {
        if spec.id.is_empty() {
            return Err("Agent ID cannot be empty".to_string());
        }
        if self.agents.contains_key(&spec.id) {
            return Err(format!("Agent {} already registered", spec.id));
        }

        let handle = AgentHandle::new(spec);
        self.agents.insert(handle.id.clone(), handle.clone());
        Ok(handle)
    }

    pub fn get(&self, id: &str) -> Option<AgentHandle> {
        self.agents.get(id).map(|h| h.clone())
    }

    pub fn list(&self) -> Vec<AgentHandle> {
        self.agents.iter().map(|h| h.clone()).collect()
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.agents.iter().map(|h| h.key().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }

    pub fn unregister(&self, id: &str) -> Option<AgentHandle> {
        self.agents.remove(id).map(|(_, h)| h)
    }

    pub fn bind_channel(&self, channel_id: &str, agent_id: &str) {
        let mut bindings = self.bindings.write();
        bindings
            .entry(channel_id.to_string())
            .or_insert_with(Vec::new)
            .push(agent_id.to_string());
    }

    pub fn agents_for_channel(&self, channel_id: &str) -> Vec<String> {
        self.bindings
            .read()
            .get(channel_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn spawn_subagent(&self, parent_id: &str, child_spec: AgentSpec) -> Result<AgentHandle, String> {
        let parent = self
            .get(parent_id)
            .ok_or_else(|| format!("Parent agent {} not found", parent_id))?;

        let child = self.register(child_spec)?;
        tracing::info!(
            parent = %parent.id,
            child = %child.id,
            "Spawned subagent"
        );
        Ok(child)
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AgentOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentOrchestrator")
            .field("count", &self.count())
            .field("ids", &self.list_ids())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::spec::{ModelSpec, AgentSpec};

    fn make_spec(id: &str) -> AgentSpec {
        AgentSpec::builder()
            .id(id)
            .name(id)
            .model(ModelSpec {
                provider: "openai".to_string(),
                name: "gpt-4".to_string(),
                base_url: None,
                api_key_env: None,
                temperature: None,
                max_tokens: None,
            })
            .instructions("test")
            .build()
            .unwrap()
    }

    #[test]
    fn test_register_and_get() {
        let orch = AgentOrchestrator::new();
        let spec = make_spec("agent1");
        let handle = orch.register(spec).unwrap();
        assert_eq!(handle.id, "agent1");

        let retrieved = orch.get("agent1").unwrap();
        assert_eq!(retrieved.id, "agent1");
    }

    #[test]
    fn test_register_duplicate() {
        let orch = AgentOrchestrator::new();
        orch.register(make_spec("agent1")).unwrap();
        let result = orch.register(make_spec("agent1"));
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_subagent() {
        let orch = AgentOrchestrator::new();
        orch.register(make_spec("parent")).unwrap();

        let child = orch.spawn_subagent("parent", make_spec("child")).unwrap();
        assert_eq!(child.id, "child");
        assert_eq!(orch.count(), 2);
    }

    #[test]
    fn test_channel_binding() {
        let orch = AgentOrchestrator::new();
        orch.bind_channel("telegram", "agent1");
        orch.bind_channel("telegram", "agent2");

        let agents = orch.agents_for_channel("telegram");
        assert_eq!(agents.len(), 2);
    }
}