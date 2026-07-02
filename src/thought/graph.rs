use super::types::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ThoughtGraph {
    nodes: DashMap<NodeId, ThoughtNode>,
    edges: DashMap<EdgeId, ThoughtEdge>,
    adj_in: Arc<RwLock<HashMap<NodeId, Vec<EdgeId>>>>,
    adj_out: Arc<RwLock<HashMap<NodeId, Vec<EdgeId>>>>,
    chains: Arc<RwLock<HashMap<ChainId, ThoughtChain>>>,
    name: String,
    description: String,
}

impl ThoughtGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            adj_in: Arc::new(RwLock::new(HashMap::new())),
            adj_out: Arc::new(RwLock::new(HashMap::new())),
            chains: Arc::new(RwLock::new(HashMap::new())),
            name: name.into(),
            description: String::new(),
        }
    }

    pub fn name(&self) -> &str { &self.name }
    pub fn set_description(&mut self, d: impl Into<String>) { self.description = d.into(); }

    pub fn add_node(&self, id: NodeId, node: ThoughtNode) {
        self.nodes.insert(id, node);
    }

    pub fn get_node(&self, id: &NodeId) -> Option<ThoughtNode> {
        self.nodes.get(id).map(|n| n.clone())
    }

    pub fn remove_node(&self, id: &NodeId) -> Option<ThoughtNode> {
        let mut adj_out = self.adj_out.write();
        let mut adj_in = self.adj_in.write();

        if let Some(out_edges) = adj_out.remove(id) {
            for eid in &out_edges {
                self.edges.remove(eid);
            }
        }
        if let Some(in_edges) = adj_in.remove(id) {
            for eid in &in_edges {
                self.edges.remove(eid);
                if let Some(edge) = self.edges.get(eid) {
                    if let Some(others) = adj_out.get_mut(&edge.from) {
                        others.retain(|e| e != eid);
                    }
                }
            }
        }

        self.nodes.remove(id).map(|(_, n)| n)
    }

    pub fn add_edge(&self, edge: ThoughtEdge) {
        let eid = edge.id;
        let from = edge.from;
        let to = edge.to;

        self.adj_out.write().entry(from).or_default().push(eid);
        self.adj_in.write().entry(to).or_default().push(eid);
        self.edges.insert(eid, edge);
    }

    pub fn add_directed_edge(&self, from: NodeId, to: NodeId) -> EdgeId {
        let edge = ThoughtEdge::new(from, to);
        let eid = edge.id;
        self.add_edge(edge);
        eid
    }

    pub fn connect(&self, from: NodeId, to: NodeId, weight: f32) -> EdgeId {
        let edge = ThoughtEdge::new(from, to).with_weight(weight);
        let eid = edge.id;
        self.add_edge(edge);
        eid
    }

    pub fn get_edge(&self, id: &EdgeId) -> Option<ThoughtEdge> {
        self.edges.get(id).map(|e| e.clone())
    }

    pub fn record_success_on_edge(&self, edge_id: &EdgeId) {
        if let Some(mut edge) = self.edges.get_mut(edge_id) {
            edge.record_success();
        }
    }

    pub fn record_failure_on_edge(&self, edge_id: &EdgeId) {
        if let Some(mut edge) = self.edges.get_mut(edge_id) {
            edge.record_failure();
        }
    }

    pub fn edges_from(&self, from: &NodeId) -> Vec<ThoughtEdge> {
        self.adj_out
            .read()
            .get(from)
            .map(|edges| {
                edges.iter()
                    .filter_map(|eid| self.edges.get(eid).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn edges_to(&self, to: &NodeId) -> Vec<ThoughtEdge> {
        self.adj_in
            .read()
            .get(to)
            .map(|edges| {
                edges.iter()
                    .filter_map(|eid| self.edges.get(eid).map(|e| e.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn best_edge_from(&self, from: &NodeId) -> Option<ThoughtEdge> {
        let mut edges = self.edges_from(from);
        edges.sort_by(|a, b| {
            b.effective_weight()
                .partial_cmp(&a.effective_weight())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        edges.into_iter().next()
    }

    pub fn all_best_edges_from(&self, from: &NodeId) -> Vec<ThoughtEdge> {
        let mut edges = self.edges_from(from);
        edges.sort_by(|a, b| {
            b.effective_weight()
                .partial_cmp(&a.effective_weight())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        edges
    }

    pub fn find_chain(&self, from: &NodeId, to: &NodeId) -> Option<ThoughtChain> {
        if from == to {
            return None;
        }

        let mut visited: HashMap<NodeId, (Option<NodeId>, EdgeId)> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        
        visited.insert(*from, (None, EdgeId::new()));
        queue.push_back(*from);

        while let Some(current) = queue.pop_front() {
            if current == *to {
                let mut path = Vec::new();
                let mut cur = *to;
                while cur != *from {
                    path.push(cur);
let (prev, _) = visited.get(&cur)?;
cur = *prev.as_ref()?;
                }
                path.push(*from);
                path.reverse();

                return Some(ThoughtChain {
                    id: ChainId::new(),
                    name: format!("chain-{}-{}", from, to),
                    nodes: path,
                    total_weight: 1.0,
                    execution_count: 0,
                    created_at: chrono::Utc::now(),
                });
            }

            for edge in self.edges_from(&current) {
                if !visited.contains_key(&edge.to) {
                    visited.insert(edge.to, (Some(current), edge.id));
                    queue.push_back(edge.to);
                }
            }
        }

        None
    }

    pub fn top_paths(&self, from: &NodeId, max_depth: usize) -> Vec<(Vec<NodeId>, f32)> {
        let mut results = Vec::new();
        let mut current_path = vec![*from];
        self.dfs_top_paths(from, &mut current_path, max_depth, 1.0, &mut results);
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn dfs_top_paths(
        &self,
        node: &NodeId,
        path: &mut Vec<NodeId>,
        max_depth: usize,
        cumulative_weight: f32,
        results: &mut Vec<(Vec<NodeId>, f32)>,
    ) {
        if path.len() > max_depth {
            return;
        }

        if path.len() > 1 {
            results.push((path.clone(), cumulative_weight));
        }

        let edges = self.all_best_edges_from(node);
        for edge in edges {
            if !path.contains(&edge.to) {
                path.push(edge.to);
                self.dfs_top_paths(
                    &edge.to,
                    path,
                    max_depth,
                    cumulative_weight * edge.effective_weight(),
                    results,
                );
                path.pop();
            }
        }
    }

    pub fn add_chain(&self, chain: ThoughtChain) {
        self.chains.write().insert(chain.id, chain);
    }

    pub fn get_chain(&self, id: &ChainId) -> Option<ThoughtChain> {
        self.chains.read().get(id).cloned()
    }

    pub fn all_chains(&self) -> Vec<ThoughtChain> {
        self.chains.read().values().cloned().collect()
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn edge_count(&self) -> usize { self.edges.len() }
    pub fn chain_count(&self) -> usize { self.chains.read().len() }

    pub fn describe(&self) -> serde_json::Value {
        let nodes: Vec<_> = self.nodes
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.key().to_string(),
                    "kind": format!("{:?}", e.value().kind()),
                })
            })
            .collect();

        let edges: Vec<_> = self.edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "from": e.value().from.to_string(),
                    "to": e.value().to.to_string(),
                    "weight": e.value().weight,
                    "effective": e.value().effective_weight(),
                    "success_rate": e.value().success_rate(),
                })
            })
            .collect();

        serde_json::json!({
            "name": self.name,
            "nodes": nodes,
            "edges": edges,
            "node_count": self.node_count(),
            "edge_count": self.edge_count(),
        })
    }

    pub fn merge_from(&self, other: &ThoughtGraph) {
        for entry in other.nodes.iter() {
            self.nodes.insert(*entry.key(), entry.value().clone());
        }
        for entry in other.edges.iter() {
            self.edges.insert(*entry.key(), entry.value().clone());
        }
        for (node_id, edge_ids) in other.adj_out.read().iter() {
            self.adj_out.write().entry(*node_id).or_default().extend(edge_ids);
        }
        for (node_id, edge_ids) in other.adj_in.read().iter() {
            self.adj_in.write().entry(*node_id).or_default().extend(edge_ids);
        }
    }

    pub fn export_chains(&self) -> Vec<ThoughtChain> {
        self.all_chains()
    }

    pub fn import_chains(&self, chains: Vec<ThoughtChain>) {
        for chain in chains {
            self.add_chain(chain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> ThoughtGraph {
        ThoughtGraph::new("test-graph")
    }

    fn make_agent_node(id: &NodeId) -> ThoughtNode {
        ThoughtNode::Agent(AgentNode {
            agent_id: id.to_string(),
            capabilities: vec!["reasoning".to_string()],
            system_prompt: None,
            max_retries: 3,
            timeout_secs: 300,
        })
    }

    #[test]
    fn test_add_and_get_node() {
        let graph = make_graph();
        let id = NodeId::new();
        graph.add_node(id, make_agent_node(&id));
        assert!(graph.get_node(&id).is_some());
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.add_directed_edge(a, b);

        assert_eq!(graph.edge_count(), 1);
        let edges = graph.edges_from(&a);
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_edge_weight_evolution() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));

        let eid = graph.connect(a, b, 0.5);
        graph.record_success_on_edge(&eid);
        
        let edge = graph.get_edge(&eid).unwrap();
        assert_eq!(edge.weight, 0.6);
        assert_eq!(edge.success_count, 1);
    }

    #[test]
    fn test_best_edge_from() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.add_node(c, make_agent_node(&c));

        graph.connect(a, b, 0.3);
        graph.connect(a, c, 0.8);

        let best = graph.best_edge_from(&a).unwrap();
        assert_eq!(best.to, c);
    }

    #[test]
    fn test_find_chain() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.add_node(c, make_agent_node(&c));

        graph.add_directed_edge(a, b);
        graph.add_directed_edge(b, c);

        let chain = graph.find_chain(&a, &c).unwrap();
        assert_eq!(chain.nodes, vec![a, b, c]);
    }

    #[test]
    fn test_top_paths() {
        let graph = make_graph();
        let a = NodeId::new();
        let b = NodeId::new();
        let c = NodeId::new();
        let d = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_node(b, make_agent_node(&b));
        graph.add_node(c, make_agent_node(&c));
        graph.add_node(d, make_agent_node(&d));

        graph.connect(a, b, 0.9);
        graph.connect(b, c, 0.8);
        graph.connect(a, d, 0.3);
        graph.connect(d, c, 0.5);

        let paths = graph.top_paths(&a, 4);
        assert!(!paths.is_empty());
        assert!(paths[0].0[0] == a);
    }

    #[test]
    fn test_remove_node_cleans_edges() {
        let graph = make_graph();
        let a = NodeId::new();
        graph.add_node(a, make_agent_node(&a));
        graph.add_directed_edge(a, a);

        graph.remove_node(&a);
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_merge_from() {
        let g1 = make_graph();
        let g2 = make_graph();

        let a = NodeId::new();
        g1.add_node(a, make_agent_node(&a));
        
        let b = NodeId::new();
        g2.add_node(b, make_agent_node(&b));

        g2.merge_from(&g1);
        assert!(g2.get_node(&a).is_some());
    }
}