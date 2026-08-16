use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ─── 节点类型 ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
    pub fn from_str(s: &str) -> Option<Self> { Uuid::parse_str(s).map(Self).ok() }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Agent,
    Tool,
    Workflow,
    Condition,
    SubGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub system_prompt: Option<String>,
    pub max_retries: u32,
    pub timeout_secs: u64,
}

impl Default for AgentNode {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            capabilities: Vec::new(),
            system_prompt: None,
            max_retries: 3,
            timeout_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNode {
    pub tool_name: String,
    pub schema: serde_json::Value,
    pub is_async: bool,
}

impl ToolNode {
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            schema: serde_json::json!({}),
            is_async: true,
        }
    }

    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = schema;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub source: WorkflowSource,
    pub definition: serde_json::Value,
    pub input_mapping: HashMap<String, String>,
    pub output_mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSource {
    Coze,
    Dify,
    N8n,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNode {
    pub condition: String,
    pub true_branch: Option<NodeId>,
    pub false_branch: Option<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ThoughtNode {
    #[serde(rename = "agent")]
    Agent(AgentNode),
    #[serde(rename = "tool")]
    Tool(ToolNode),
    #[serde(rename = "workflow")]
    Workflow(WorkflowNode),
    #[serde(rename = "condition")]
    Condition(ConditionNode),
    #[serde(rename = "subgraph")]
    SubGraph(SubGraphNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphNode {
    pub graph_id: NodeId,
}

impl ThoughtNode {
    pub fn kind(&self) -> NodeKind {
        match self {
            ThoughtNode::Agent(_) => NodeKind::Agent,
            ThoughtNode::Tool(_) => NodeKind::Tool,
            ThoughtNode::Workflow(_) => NodeKind::Workflow,
            ThoughtNode::Condition(_) => NodeKind::Condition,
            ThoughtNode::SubGraph(_) => NodeKind::SubGraph,
        }
    }
}

// ─── 边 ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub weight: f32,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);
impl EdgeId { pub fn new() -> Self { Self(Uuid::new_v4()) } }

impl ThoughtEdge {
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            weight: 0.5,
            success_count: 0,
            failure_count: 0,
            last_success: None,
            condition: None,
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    pub fn success_rate(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 { return 0.0; }
        self.success_count as f32 / total as f32
    }

    pub fn effective_weight(&self) -> f32 {
        let sr = self.success_rate();
        if sr > 0.0 { sr * self.weight } else { self.weight * 0.5 }
    }

    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.last_success = Some(chrono::Utc::now());
        self.weight = (self.weight + 0.1).min(1.0);
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.weight = (self.weight - 0.05).max(0.05);
    }
}

// ─── 链 ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtChain {
    pub id: ChainId,
    pub name: String,
    pub nodes: Vec<NodeId>,
    pub total_weight: f32,
    pub execution_count: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub Uuid);
impl ChainId { pub fn new() -> Self { Self(Uuid::new_v4()) } }

impl ThoughtChain {
    pub fn builder() -> ThoughtChainBuilder {
        ThoughtChainBuilder::default()
    }
}

#[derive(Default)]
pub struct ThoughtChainBuilder {
    name: Option<String>,
    nodes: Vec<NodeId>,
}

impl ThoughtChainBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn add_node(mut self, id: NodeId) -> Self {
        self.nodes.push(id);
        self
    }
    pub fn build(self) -> ThoughtChain {
        ThoughtChain {
            id: ChainId::new(),
            name: self.name.unwrap_or_default(),
            nodes: self.nodes,
            total_weight: 1.0,
            execution_count: 0,
            created_at: chrono::Utc::now(),
        }
    }
}

// ─── 调度记录 ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRecord {
    pub task_id: String,
    pub chain_id: ChainId,
    pub path: Vec<NodeId>,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_creation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_edge_creation() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = ThoughtEdge::new(from, to).with_weight(0.7);
        assert_eq!(edge.weight, 0.7);
        assert_eq!(edge.success_count, 0);
    }

    #[test]
    fn test_edge_record_success_increases_weight() {
        let mut edge = ThoughtEdge::new(NodeId::new(), NodeId::new()).with_weight(0.5);
        edge.record_success();
        assert_eq!(edge.weight, 0.6);
        assert_eq!(edge.success_count, 1);
    }

    #[test]
    fn test_edge_record_failure_decreases_weight() {
        let mut edge = ThoughtEdge::new(NodeId::new(), NodeId::new()).with_weight(0.5);
        edge.record_failure();
        assert_eq!(edge.weight, 0.45);
        assert_eq!(edge.failure_count, 1);
    }

    #[test]
    fn test_edge_weight_clamped() {
        let mut edge = ThoughtEdge::new(NodeId::new(), NodeId::new());
        for _ in 0..20 { edge.record_success(); }
        assert!(edge.weight <= 1.0);

        let mut edge2 = ThoughtEdge::new(NodeId::new(), NodeId::new());
        for _ in 0..20 { edge2.record_failure(); }
        assert!(edge2.weight >= 0.05);
    }

    #[test]
    fn test_edge_effective_weight() {
        let mut edge = ThoughtEdge::new(NodeId::new(), NodeId::new()).with_weight(0.8);
        edge.record_success();
        edge.record_success();
        edge.record_failure();

        let effective = edge.effective_weight();
        assert!(effective > 0.0 && effective <= 1.0);
    }

    #[test]
    fn test_chain_builder() {
        let chain = ThoughtChain::builder()
            .name("test-chain")
            .add_node(NodeId::new())
            .add_node(NodeId::new())
            .build();

        assert_eq!(chain.name, "test-chain");
        assert_eq!(chain.nodes.len(), 2);
    }

    #[test]
    fn test_tool_node_creation() {
        let tool = ToolNode::new("search").with_schema(serde_json::json!({"type": "object"}));
        assert_eq!(tool.tool_name, "search");
        assert!(tool.is_async);
    }

    #[test]
    fn test_workflow_source() {
        assert_eq!(WorkflowSource::Coze as i32, 0);
        assert_ne!(WorkflowSource::Coze, WorkflowSource::Dify);
    }
}