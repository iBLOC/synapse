use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

pub trait Channel: Send + Sync {
    fn report(&self, node_key: &str, value: serde_json::Value) -> Result<(), String>;
    fn get(&self, node_key: &str) -> Result<Option<serde_json::Value>, String>;
    fn mark_done(&self, node_key: &str) -> Result<(), String>;
    fn done_nodes(&self) -> Vec<String>;
    fn all_predecessors_done(&self, node_key: &str, deps: &[String]) -> bool;
    fn any_predecessor_done(&self, node_key: &str, deps: &[String]) -> bool;
    fn pending_predecessors(&self, node_key: &str, deps: &[String]) -> Vec<String>;
    fn reset(&self) -> Result<(), String>;
}

pub struct DagChannel {
    data: RwLock<HashMap<String, serde_json::Value>>,
    done: RwLock<HashSet<String>>,
    pending: RwLock<HashMap<String, HashSet<String>>>,
}

impl DagChannel {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            pending: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_dependencies(deps: HashMap<String, Vec<String>>) -> Self {
        let pending: HashMap<String, HashSet<String>> = deps
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            pending: RwLock::new(pending),
        }
    }
}

impl Default for DagChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for DagChannel {
    fn report(&self, node_key: &str, value: serde_json::Value) -> Result<(), String> {
        self.data.write().insert(node_key.to_string(), value);
        Ok(())
    }

    fn get(&self, node_key: &str) -> Result<Option<serde_json::Value>, String> {
        Ok(self.data.read().get(node_key).cloned())
    }

    fn mark_done(&self, node_key: &str) -> Result<(), String> {
        self.done.write().insert(node_key.to_string());
        let mut pending = self.pending.write();
        for deps in pending.values_mut() {
            deps.remove(node_key);
        }
        Ok(())
    }

    fn done_nodes(&self) -> Vec<String> {
        self.done.read().iter().cloned().collect()
    }

    fn all_predecessors_done(&self, node_key: &str, _deps: &[String]) -> bool {
        self.pending
            .read()
            .get(node_key)
            .map(|p| p.is_empty())
            .unwrap_or(true)
    }

    fn any_predecessor_done(&self, _node_key: &str, _deps: &[String]) -> bool {
        false
    }

    fn pending_predecessors(&self, node_key: &str, _deps: &[String]) -> Vec<String> {
        self.pending
            .read()
            .get(node_key)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn reset(&self) -> Result<(), String> {
        self.data.write().clear();
        self.done.write().clear();
        Ok(())
    }
}

pub struct PregelChannel {
    data: RwLock<HashMap<String, serde_json::Value>>,
    done: RwLock<HashSet<String>>,
    waiting: RwLock<HashMap<String, HashSet<String>>>,
    step: std::sync::atomic::AtomicU64,
}

impl PregelChannel {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            waiting: RwLock::new(HashMap::new()),
            step: 0u64.into(),
        }
    }

    pub fn with_dependencies(deps: HashMap<String, Vec<String>>) -> Self {
        let waiting: HashMap<String, HashSet<String>> = deps
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            waiting: RwLock::new(waiting),
            step: 0u64.into(),
        }
    }

    pub fn current_step(&self) -> u64 {
        self.step.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn increment_step(&self) {
        self.step.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Default for PregelChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel for PregelChannel {
    fn report(&self, node_key: &str, value: serde_json::Value) -> Result<(), String> {
        self.data.write().insert(node_key.to_string(), value);
        Ok(())
    }

    fn get(&self, node_key: &str) -> Result<Option<serde_json::Value>, String> {
        Ok(self.data.read().get(node_key).cloned())
    }

    fn mark_done(&self, node_key: &str) -> Result<(), String> {
        self.done.write().insert(node_key.to_string());
        let dependents: Vec<String> = self
            .waiting
            .read()
            .get(node_key)
            .map(|d| d.iter().cloned().collect())
            .unwrap_or_default();
        let mut waiting = self.waiting.write();
        for d in &dependents {
            if let Some(w) = waiting.get_mut(d) {
                w.remove(node_key);
            }
        }
        Ok(())
    }

    fn done_nodes(&self) -> Vec<String> {
        self.done.read().iter().cloned().collect()
    }

    fn all_predecessors_done(&self, _node_key: &str, _deps: &[String]) -> bool {
        false
    }

    fn any_predecessor_done(&self, node_key: &str, deps: &[String]) -> bool {
        deps.iter().any(|dep| self.done.read().contains(dep)) || self.done.read().contains(node_key)
    }

    fn pending_predecessors(&self, node_key: &str, _deps: &[String]) -> Vec<String> {
        self.waiting
            .read()
            .get(node_key)
            .map(|p| p.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn reset(&self) -> Result<(), String> {
        self.data.write().clear();
        self.done.write().clear();
        self.step.store(0, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_channel_all_predecessor() {
        let mut deps = HashMap::new();
        deps.insert("node2".to_string(), vec!["node1".to_string()]);
        deps.insert(
            "node3".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );

        let ch = DagChannel::with_dependencies(deps);

        assert!(!ch.all_predecessors_done("node2", &["node1".to_string()]));
        ch.mark_done("node1").unwrap();
        assert!(ch.all_predecessors_done("node2", &["node1".to_string()]));
    }

    #[test]
    fn test_dag_channel_mark_done() {
        let ch = DagChannel::new();
        ch.report("node1", serde_json::json!("value")).unwrap();
        assert_eq!(ch.get("node1").unwrap(), Some(serde_json::json!("value")));
        ch.mark_done("node1").unwrap();
        assert!(ch.done_nodes().contains(&"node1".to_string()));
    }

    #[test]
    fn test_pregel_channel_any_predecessor() {
        let ch = PregelChannel::new();
        assert!(!ch.any_predecessor_done("node1", &[]));
        ch.mark_done("node1").unwrap();
        assert!(ch.any_predecessor_done("node1", &[]));
    }

    #[test]
    fn test_pregel_channel_step_counter() {
        let ch = PregelChannel::new();
        assert_eq!(ch.current_step(), 0);
        ch.increment_step();
        assert_eq!(ch.current_step(), 1);
    }
}
