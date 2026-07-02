use super::graph::ThoughtGraph;
use super::types::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TaskRequest {
    pub task_id: String,
    pub description: String,
    pub input: serde_json::Value,
    pub preferred_nodes: Vec<NodeId>,
    pub max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct TaskAssignment {
    pub task_id: String,
    pub chain: ThoughtChain,
    pub estimated_weight: f32,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub chain_id: ChainId,
    pub path: Vec<NodeId>,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
}

pub struct WeightedScheduler {
    graph: Arc<ThoughtGraph>,
    history: Arc<RwLock<Vec<SchedulingRecord>>>,
    chains: Arc<RwLock<HashMap<String, Vec<ThoughtChain>>>>,
}

impl WeightedScheduler {
    pub fn new(graph: Arc<ThoughtGraph>) -> Self {
        Self {
            graph,
            history: Arc::new(RwLock::new(Vec::new())),
            chains: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn schedule(&self, task: &TaskRequest) -> Option<TaskAssignment> {
        if !task.preferred_nodes.is_empty() {
            for start in &task.preferred_nodes {
                if let Some(chain) = self.find_best_chain(start, task) {
                    return Some(TaskAssignment {
                        task_id: task.task_id.clone(),
                        estimated_weight: chain.total_weight,
                        chain,
                    });
                }
            }
        }

        let start = task.preferred_nodes.first().copied()?;
for entry in self.graph.top_paths(&start, task.max_depth) {
    let (path, weight) = entry;
            let chain = ThoughtChain {
                id: ChainId::new(),
                name: format!("auto-{}", task.task_id),
                nodes: path,
                total_weight: weight,
                execution_count: 0,
                created_at: chrono::Utc::now(),
            };
            return Some(TaskAssignment {
                task_id: task.task_id.clone(),
                estimated_weight: weight,
                chain,
            });
        }

        None
    }

    fn find_best_chain(&self, start: &NodeId, task: &TaskRequest) -> Option<ThoughtChain> {
        let paths = self.graph.top_paths(start, task.max_depth);
        paths.into_iter().next().map(|(path, weight)| ThoughtChain {
            id: ChainId::new(),
            name: format!("best-{}", task.task_id),
            nodes: path,
            total_weight: weight,
            execution_count: 0,
            created_at: chrono::Utc::now(),
        })
    }

    pub fn record_result(&self, result: &TaskResult) {
        let record = SchedulingRecord {
            task_id: result.task_id.clone(),
            chain_id: result.chain_id,
            path: result.path.clone(),
            success: result.success,
            duration_ms: result.duration_ms,
            timestamp: chrono::Utc::now(),
        };

        if result.success {
            for i in 0..result.path.len().saturating_sub(1) {
                let from = result.path[i];
                let to = result.path[i + 1];
                for edge in self.graph.edges_from(&from) {
                    if edge.to == to {
                        self.graph.record_success_on_edge(&edge.id);
                        break;
                    }
                }
            }
        } else {
            for i in 0..result.path.len().saturating_sub(1) {
                let from = result.path[i];
                let to = result.path[i + 1];
                for edge in self.graph.edges_from(&from) {
                    if edge.to == to {
                        self.graph.record_failure_on_edge(&edge.id);
                        break;
                    }
                }
            }
        }

        self.history.write().push(record);
    }

    pub fn record_chain_success(&self, chain: &ThoughtChain) {
        for i in 0..chain.nodes.len().saturating_sub(1) {
            self.graph.connect(chain.nodes[i], chain.nodes[i + 1], 0.5);
            let edges = self.graph.edges_from(&chain.nodes[i]);
            if let Some(edge) = edges.iter().find(|e| e.to == chain.nodes[i + 1]) {
                self.graph.record_success_on_edge(&edge.id);
            }
        }
    }

    pub fn record_chain_failure(&self, chain: &ThoughtChain) {
        for i in 0..chain.nodes.len().saturating_sub(1) {
            let edges = self.graph.edges_from(&chain.nodes[i]);
            if let Some(edge) = edges.iter().find(|e| e.to == chain.nodes[i + 1]) {
                self.graph.record_failure_on_edge(&edge.id);
            }
        }
    }

    pub fn switch_to_alternative(&self, task: &TaskRequest, failed_chain: &ThoughtChain) -> Option<TaskAssignment> {
        let last_node_idx = failed_chain.nodes.len().saturating_sub(1);
        if last_node_idx == 0 {
            return None;
        }

        let branch_point = failed_chain.nodes[last_node_idx - 1];
        let failing_node = failed_chain.nodes[last_node_idx];

        let mut alt_edges = self.graph.edges_from(&branch_point);
        alt_edges.retain(|e| e.to != failing_node);
        alt_edges.sort_by(|a, b| {
            b.effective_weight()
                .partial_cmp(&a.effective_weight())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(best_alt) = alt_edges.first() {
            let mut alt_path = failed_chain.nodes[..last_node_idx].to_vec();
            alt_path.push(best_alt.to);

            let chain = ThoughtChain {
                id: ChainId::new(),
                name: format!("alt-{}", task.task_id),
                nodes: alt_path,
                total_weight: best_alt.effective_weight(),
                execution_count: 0,
                created_at: chrono::Utc::now(),
            };

            return Some(TaskAssignment {
                task_id: task.task_id.clone(),
                estimated_weight: chain.total_weight,
                chain,
            });
        }

        None
    }

    pub fn history(&self) -> Vec<SchedulingRecord> {
        self.history.read().clone()
    }

    pub fn recent_successes(&self, limit: usize) -> Vec<SchedulingRecord> {
        let mut all = self.history();
        all.retain(|r| r.success);
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(limit);
        all
    }

    pub fn best_chain_for_pattern(&self, task_pattern: &str) -> Option<ThoughtChain> {
        let history = self.history.read();
        
        let pattern_lower = task_pattern.to_lowercase();
        let successful: Vec<_> = history
            .iter()
            .filter(|r| r.success && r.path.len() >= 2)
            .collect();

        if let Some(best) = successful.first() {
            return Some(ThoughtChain {
                id: ChainId::new(),
                name: format!("pattern-{}", task_pattern),
                nodes: best.path.clone(),
                total_weight: 1.0,
                execution_count: 0,
                created_at: chrono::Utc::now(),
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> Arc<ThoughtGraph> {
        Arc::new(ThoughtGraph::new("test"))
    }

    fn make_task(id: &str, start_node: NodeId) -> TaskRequest {
        TaskRequest {
            task_id: id.to_string(),
            description: "test task".to_string(),
            input: serde_json::json!({}),
            preferred_nodes: vec![start_node],
            max_depth: 5,
        }
    }

    fn make_agent_node(id: &NodeId) -> ThoughtNode {
        ThoughtNode::Agent(AgentNode {
            agent_id: id.to_string(),
            capabilities: vec!["test".to_string()],
            system_prompt: None,
            max_retries: 3,
            timeout_secs: 300,
        })
    }

    #[test]
    fn test_scheduler_creation() {
        let graph = make_graph();
        let scheduler = WeightedScheduler::new(graph);
        assert!(scheduler.history().is_empty());
    }

    #[test]
    fn test_schedule_basic() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.connect(a, b, 0.8);

        let scheduler = WeightedScheduler::new(graph);
        let task = make_task("t1", a);
        let assignment = scheduler.schedule(&task).unwrap();
        assert!(assignment.chain.nodes.contains(&a));
    }

    #[test]
    fn test_record_result_updates_weights() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.connect(a, b, 0.5);

        let scheduler = WeightedScheduler::new(graph.clone());

        let result = TaskResult {
            task_id: "t1".to_string(),
            chain_id: ChainId::new(),
            path: vec![a, b],
            success: true,
            output: serde_json::json!("ok"),
            duration_ms: 100,
        };
        scheduler.record_result(&result);

        let edge = graph.edges_from(&a).first().cloned().unwrap();
        assert_eq!(edge.weight, 0.6);
        assert_eq!(edge.success_count, 1);
    }

    #[test]
    fn test_switch_to_alternative() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.add_node(c, make_agent_node(&c));
        graph.connect(a, b, 0.8);
        graph.connect(a, c, 0.3);

        let failed_chain = ThoughtChain::builder()
            .add_node(a)
            .add_node(b)
            .build();

        let scheduler = WeightedScheduler::new(graph);
        let task = make_task("t1", a);
        let alt = scheduler.switch_to_alternative(&task, &failed_chain).unwrap();
        assert!(alt.chain.nodes.contains(&c));
    }

    #[test]
    fn test_record_chain_success_bumps_weights() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));

        let chain = ThoughtChain::builder()
            .add_node(a)
            .add_node(b)
            .build();

        let scheduler = WeightedScheduler::new(graph.clone());
        for _ in 0..3 {
            scheduler.record_chain_success(&chain);
        }

        let edge = graph.edges_from(&a).first().cloned().unwrap();
        assert!(edge.weight > 0.5);
    }
}