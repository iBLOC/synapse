use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

/// Channel trait — 节点间数据传递的核心抽象
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

/// DAG Channel — AllPredecessor 模式（无循环）
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
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect();
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            pending: RwLock::new(pending),
        }
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

/// Pregel Channel — AnyPredecessor 模式（支持循环）
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
            step: 0.into(),
        }
    }

    pub fn with_dependencies(deps: HashMap<String, Vec<String>>) -> Self {
        let waiting: HashMap<String, HashSet<String>> = deps
            .iter()
            .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
            .collect();
        Self {
            data: RwLock::new(HashMap::new()),
            done: RwLock::new(HashSet::new()),
            waiting: RwLock::new(waiting),
            step: 0.into(),
        }
    }

    pub fn current_step(&self) -> u64 {
        self.step.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn increment_step(&self) {
        self.step.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        
        // Collect dependent keys first to avoid borrow issues
        let dependent_keys: Vec<String> = {
            let waiting = self.waiting.read();
            waiting
                .values()
                .filter(|deps| deps.contains(node_key))
                .flat_map(|deps| deps.iter())
                .filter(|d| *d != node_key)
                .cloned()
                .collect()
        };
        
        // Remove from waiting sets
        let mut waiting = self.waiting.write();
        for key in dependent_keys {
            if let Some(deps) = waiting.get_mut(&key) {
                deps.remove(node_key);
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
        if deps.is_empty() {
            self.done.read().contains(node_key)
        } else {
            deps.iter().any(|d| self.done.read().contains(d.as_str()))
        }
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
    use std::collections::HashMap;

    #[test]
    fn test_dag_channel_all_predecessor() {
        let mut deps = HashMap::new();
        deps.insert("B".to_string(), vec!["A".to_string()]);
        deps.insert("C".to_string(), vec!["B".to_string()]);

        let channel = DagChannel::with_dependencies(deps);

        assert!(!channel.all_predecessors_done("B", &["A".to_string()]));
        assert!(!channel.all_predecessors_done("C", &["B".to_string()]));

        channel.report("A", serde_json::json!("value_a")).unwrap();
        channel.mark_done("A").unwrap();

        assert!(channel.all_predecessors_done("B", &["A".to_string()]));

        channel.report("B", serde_json::json!("value_b")).unwrap();
        channel.mark_done("B").unwrap();

        assert!(channel.all_predecessors_done("C", &["B".to_string()]));
    }

    #[test]
    fn test_dag_channel_mark_done() {
        let channel = DagChannel::new();

        assert!(channel.done_nodes().is_empty());

        channel.report("node1", serde_json::json!("test")).unwrap();
        channel.mark_done("node1").unwrap();

        assert!(channel.done_nodes().contains(&"node1".to_string()));
        assert_eq!(channel.get("node1").unwrap(), Some(serde_json::json!("test")));

        let mut deps = HashMap::new();
        deps.insert("node2".to_string(), vec!["node1".to_string()]);
        let channel2 = DagChannel::with_dependencies(deps);

        let pending = channel2.pending_predecessors("node2", &["node1".to_string()]);
        assert!(pending.contains(&"node1".to_string()));

        channel2.mark_done("node1").unwrap();
        assert!(channel2.pending_predecessors("node2", &["node1".to_string()]).is_empty());
    }

    #[test]
    fn test_pregel_channel_any_predecessor() {
        // Create Pregel with dependencies: B depends on A, C depends on A and B
        let mut deps = HashMap::new();
        deps.insert("B".to_string(), vec!["A".to_string()]);
        deps.insert("C".to_string(), vec!["A".to_string(), "B".to_string()]);

        let channel = PregelChannel::with_dependencies(deps);

        // Initially, no predecessors are done for any node
        assert!(!channel.any_predecessor_done("A", &[]));
        assert!(!channel.any_predecessor_done("B", &["A".to_string()]));
        assert!(!channel.any_predecessor_done("C", &["A".to_string(), "B".to_string()]));

        // Report and mark A done
        channel.report("A", serde_json::json!("value_a")).unwrap();
        channel.mark_done("A").unwrap();

        // Now A is done, so B and C have at least one predecessor done
        assert!(channel.any_predecessor_done("A", &[]));
        assert!(channel.any_predecessor_done("B", &["A".to_string()]));
        assert!(channel.any_predecessor_done("C", &["A".to_string(), "B".to_string()]));

        // Report and mark B done
        channel.report("B", serde_json::json!("value_b")).unwrap();
        channel.mark_done("B").unwrap();

        // Now C has both predecessors done
        assert!(channel.any_predecessor_done("C", &["A".to_string(), "B".to_string()]));
    }

    #[test]
    fn test_pregel_channel_step_counter() {
        let channel = PregelChannel::new();

        // Initially step is 0
        assert_eq!(channel.current_step(), 0);

        // Increment step
        channel.increment_step();
        assert_eq!(channel.current_step(), 1);

        channel.increment_step();
        assert_eq!(channel.current_step(), 2);

        // Test reset
        channel.reset().unwrap();
        assert_eq!(channel.current_step(), 0);

        // Test that data and done are also cleared on reset
        channel.report("node1", serde_json::json!("test")).unwrap();
        channel.mark_done("node1").unwrap();
        channel.increment_step();

        assert_eq!(channel.current_step(), 1);
        assert!(channel.get("node1").unwrap().is_some());

        channel.reset().unwrap();
        assert!(channel.get("node1").unwrap().is_none());
        assert!(channel.done_nodes().is_empty());
    }
}
