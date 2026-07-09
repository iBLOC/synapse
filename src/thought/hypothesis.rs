use super::types::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    Proposed,
    Dispatched,
    Running,
    Succeeded,
    Failed,
    Pruned,
    Merged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArborAction {
    Prune,
    Continue,
    Merge,
    Stop,
    Backtrack,
}

#[derive(Debug, Clone)]
pub struct HypothesisNode {
    pub id: NodeId,
    pub parent_id: Option<NodeId>,
    pub description: String,
    pub status: HypothesisStatus,
    pub evidence_score: f32,
    pub dev_score: Option<f32>,
    pub test_score: Option<f32>,
    pub children: Vec<NodeId>,
    pub insights: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl HypothesisNode {
    pub fn new(id: NodeId, description: impl Into<String>) -> Self {
        Self {
            id,
            parent_id: None,
            description: description.into(),
            status: HypothesisStatus::Proposed,
            evidence_score: 0.0,
            dev_score: None,
            test_score: None,
            children: Vec::new(),
            insights: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_parent(mut self, parent_id: NodeId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn can_be_dispatched(&self) -> bool {
        matches!(self.status, HypothesisStatus::Proposed)
            || matches!(self.status, HypothesisStatus::Failed)
    }
}

pub struct HypothesisTree {
    pub root_id: NodeId,
    nodes: Arc<RwLock<HashMap<NodeId, HypothesisNode>>>,
    objective: String,
    dev_evaluator: String,
    test_evaluator: String,
    max_depth: usize,
    max_branches: usize,
}

impl HypothesisTree {
    pub fn new(
        root_description: impl Into<String>,
        objective: impl Into<String>,
    ) -> Self {
        let root = HypothesisNode::new(NodeId::new(), root_description);
        let root_id = root.id;
        let mut nodes = HashMap::new();
        nodes.insert(root_id, root);

        Self {
            root_id,
            nodes: Arc::new(RwLock::new(nodes)),
            objective: objective.into(),
            dev_evaluator: String::new(),
            test_evaluator: String::new(),
            max_depth: 5,
            max_branches: 3,
        }
    }

    pub fn with_evaluators(mut self, dev: impl Into<String>, test: impl Into<String>) -> Self {
        self.dev_evaluator = dev.into();
        self.test_evaluator = test.into();
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_max_branches(mut self, branches: usize) -> Self {
        self.max_branches = branches;
        self
    }

    pub fn root(&self) -> HypothesisNode {
        self.get_node(&self.root_id).unwrap()
    }

    pub fn get_node(&self, id: &NodeId) -> Option<HypothesisNode> {
        self.nodes.read().get(id).cloned()
    }

    pub fn depth(&self, id: &NodeId) -> usize {
        let mut depth = 0;
        let mut current = *id;
        while let Some(node) = self.get_node(&current) {
            match node.parent_id {
                Some(parent) => {
                    depth += 1;
                    current = parent;
                }
                None => break,
            }
        }
        depth
    }

    pub fn ideate(&self, parent_id: &NodeId, descriptions: Vec<String>) -> Vec<NodeId> {
        let parent_depth = self.depth(parent_id);
        if parent_depth >= self.max_depth {
            return Vec::new();
        }

        let parent = match self.get_node(parent_id) {
            Some(p) => p,
            None => return Vec::new(),
        };
        let _ = parent;

        let mut new_ids = Vec::new();
        let mut nodes = self.nodes.write();

        for desc in descriptions {
            let child = HypothesisNode::new(NodeId::new(), desc)
                .with_parent(*parent_id);
            let child_id = child.id;
            new_ids.push(child_id);
            nodes.insert(child_id, child);
        }

        if let Some(parent) = nodes.get_mut(parent_id) {
            parent.children.extend(&new_ids);
        }

        new_ids
    }

    pub fn dispatch(&self, node_id: &NodeId) -> Result<(), String> {
        let mut nodes = self.nodes.write();
        let node = nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("Node not found: {}", node_id))?;

        if !node.can_be_dispatched() {
            return Err(format!("Cannot dispatch node {:?}: status={:?}", node_id, node.status));
        }

        node.status = HypothesisStatus::Dispatched;
        Ok(())
    }

    pub fn record_result(
        &self,
        node_id: &NodeId,
        dev_score: f32,
        test_score: f32,
        insights: Vec<String>,
    ) -> Result<(ArborAction, f32), String> {
        let depth = self.depth(node_id);

        let mut nodes = self.nodes.write();
        let node = nodes
            .get_mut(node_id)
            .ok_or_else(|| format!("Node not found: {}", node_id))?;

        node.dev_score = Some(dev_score);
        node.test_score = Some(test_score);
        node.insights = insights;
        node.evidence_score = test_score;

        let action = self.decide_inner(node, depth);

        match action {
            ArborAction::Merge => node.status = HypothesisStatus::Merged,
            ArborAction::Continue => node.status = HypothesisStatus::Succeeded,
            ArborAction::Prune => node.status = HypothesisStatus::Pruned,
            ArborAction::Backtrack => node.status = HypothesisStatus::Failed,
            ArborAction::Stop => node.status = HypothesisStatus::Succeeded,
        }

        Ok((action, node.evidence_score))
    }

    fn decide_inner(&self, node: &HypothesisNode, depth: usize) -> ArborAction {
        let dev = node.dev_score.unwrap_or(0.0);
        let test = node.test_score.unwrap_or(0.0);

        if test > 0.8 { return ArborAction::Merge; }
        if dev > 0.7 { return ArborAction::Continue; }

        if dev <= 0.3 || test <= 0.3 { return ArborAction::Prune; }

        if depth < self.max_depth && node.children.len() < self.max_branches {
            return ArborAction::Continue;
        }

        ArborAction::Prune
    }

    pub fn backpropagate(&self, node_id: &NodeId, insight: String) {
        let mut current = Some(*node_id);
        let mut nodes = self.nodes.write();

        while let Some(id) = current {
            if let Some(node) = nodes.get_mut(&id) {
                node.insights.push(insight.clone());
                node.evidence_score = (node.evidence_score + 0.05).min(1.0);
                current = node.parent_id;
            } else {
                break;
            }
        }
    }

    pub fn leaves(&self) -> Vec<HypothesisNode> {
        self.nodes
            .read()
            .values()
            .filter(|n| n.is_leaf())
            .cloned()
            .collect()
    }

    pub fn dispatched_nodes(&self) -> Vec<HypothesisNode> {
        self.nodes
            .read()
            .values()
            .filter(|n| matches!(n.status, HypothesisStatus::Dispatched))
            .cloned()
            .collect()
    }

    pub fn succeeded_nodes(&self) -> Vec<HypothesisNode> {
        self.nodes
            .read()
            .values()
            .filter(|n| matches!(n.status, HypothesisStatus::Succeeded | HypothesisStatus::Merged))
            .cloned()
            .collect()
    }

    pub fn best_path(&self) -> Vec<HypothesisNode> {
        let nodes = self.nodes.read();
        let mut current = self.root_id;
        let mut path = Vec::new();

        loop {
            let node = match nodes.get(&current) {
                Some(n) => n.clone(),
                None => break,
            };
            path.push(node.clone());

            let best_child = node
                .children
                .iter()
                .filter_map(|cid| nodes.get(cid))
                .max_by(|a, b| {
                    a.evidence_score
                        .partial_cmp(&b.evidence_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            match best_child {
                Some(child) => current = child.id,
                None => break,
            }
        }

        path
    }

    pub fn describe(&self) -> serde_json::Value {
        let nodes = self.nodes.read();
        let node_list: Vec<_> = nodes
            .values()
            .map(|n| {
                serde_json::json!({
                    "id": n.id.to_string(),
                    "parent": n.parent_id.map(|p| p.to_string()),
                    "description": n.description,
                    "status": format!("{:?}", n.status),
                    "evidence": n.evidence_score,
                    "dev_score": n.dev_score,
                    "test_score": n.test_score,
                    "children": n.children.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                })
            })
            .collect();

        serde_json::json!({
            "objective": self.objective,
            "root": self.root_id.to_string(),
            "node_count": nodes.len(),
            "nodes": node_list,
            "max_depth": self.max_depth,
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.read().len()
    }

    pub fn leaf_count(&self) -> usize {
        self.leaves().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tree() {
        let tree = HypothesisTree::new("Initial design", "Improve benchmark score by 5%");
        let root = tree.root();
        assert_eq!(root.description, "Initial design");
        assert!(root.is_leaf());
    }

    #[test]
    fn test_ideate_creates_children() {
        let tree = HypothesisTree::new("Root", "Objective");
        let root_id = tree.root_id;

        let children = tree.ideate(&root_id, vec![
            "Try approach A".to_string(),
            "Try approach B".to_string(),
            "Try approach C".to_string(),
        ]);

        assert_eq!(children.len(), 3);
        let root = tree.root();
        assert_eq!(root.children.len(), 3);
    }

    #[test]
    fn test_dispatch_and_record() {
        let tree = HypothesisTree::new("Root", "Obj");
        let children = tree.ideate(&tree.root_id, vec!["Test hypothesis".to_string()]);

        tree.dispatch(&children[0]).unwrap();
        let (action, score) = tree.record_result(
            &children[0],
            0.75,
            0.85,
            vec!["It works!".to_string()],
        ).unwrap();

        assert!(matches!(action, ArborAction::Merge));
        assert!(score > 0.8);
    }

    #[test]
    fn test_prune_on_low_score() {
        let tree = HypothesisTree::new("Root", "Obj");
        let children = tree.ideate(&tree.root_id, vec!["Bad hypothesis".to_string()]);

        tree.dispatch(&children[0]).unwrap();
        let (action, _) = tree.record_result(
            &children[0],
            0.3,
            0.4,
            vec!["Didn't work".to_string()],
        ).unwrap();

        assert!(matches!(action, ArborAction::Prune));
    }

    #[test]
    fn test_backpropagate() {
        let tree = HypothesisTree::new("Root", "Obj");
        let children = tree.ideate(&tree.root_id, vec!["H1".to_string()]);
        let grandchildren = tree.ideate(&children[0], vec!["H1.1".to_string()]);

        tree.backpropagate(&grandchildren[0], "Found something interesting".to_string());

        let root = tree.root();
        assert!(!root.insights.is_empty());
        assert!(root.evidence_score > 0.0);
    }

    #[test]
    fn test_best_path() {
        let tree = HypothesisTree::new("Root", "Obj");
        let children = tree.ideate(&tree.root_id, vec!["Good".to_string(), "Better".to_string()]);

        tree.dispatch(&children[0]).unwrap();
        tree.record_result(&children[0], 0.3, 0.4, vec!["ok".to_string()]).unwrap();

        tree.dispatch(&children[1]).unwrap();
        tree.record_result(&children[1], 0.9, 0.95, vec!["great".to_string()]).unwrap();

        let path = tree.best_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[1].id, children[1]);
    }

    #[test]
    fn test_max_depth_prevents_ideation() {
        let tree = HypothesisTree::new("Root", "Obj").with_max_depth(2);
        let c1 = tree.ideate(&tree.root_id, vec!["H1".to_string()]);
        let c2 = tree.ideate(&c1[0], vec!["H1.1".to_string()]);
        let c3 = tree.ideate(&c2[0], vec!["H1.1.1".to_string()]);

        assert!(c3.is_empty());
    }

    #[test]
    fn test_leaf_count() {
        let tree = HypothesisTree::new("Root", "Obj");
        assert_eq!(tree.leaf_count(), 1);

        let children = tree.ideate(&tree.root_id, vec!["A".to_string(), "B".to_string()]);
        let _ = children;
        assert_eq!(tree.leaf_count(), 2);
    }
}