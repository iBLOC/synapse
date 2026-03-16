use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentProtocol {
    A2A,
    OpenCode,
    OpenGlow,
    MCP,
    Custom(String),
}

impl AgentProtocol {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "a2a" => AgentProtocol::A2A,
            "opencode" => AgentProtocol::OpenCode,
            "openglow" => AgentProtocol::OpenGlow,
            "mcp" => AgentProtocol::MCP,
            _ => AgentProtocol::Custom(s.to_string()),
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            AgentProtocol::A2A => "a2a",
            AgentProtocol::OpenCode => "opencode",
            AgentProtocol::OpenGlow => "openglow",
            AgentProtocol::MCP => "mcp",
            AgentProtocol::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEndpoint {
    pub url: String,
    pub protocol: AgentProtocol,
    pub capabilities: Vec<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub task_id: String,
    pub messages: Vec<ProtocolMessage>,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub role: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub task_id: String,
    pub status: TaskStatus,
    pub messages: Vec<ProtocolMessage>,
    pub result: serde_json::Value,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
}

pub type AdapterResult<T> = std::result::Result<T, String>;

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn protocol(&self) -> AgentProtocol;
    
    async fn send_task(&self, endpoint: &AgentEndpoint, request: &AgentRequest) -> AdapterResult<AgentResponse>;
    
    async fn get_task_status(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<AgentResponse>;
    
    async fn cancel_task(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<()>;
    
    async fn discover_agent(&self, url: &str) -> AdapterResult<AgentEndpoint>;
}

pub type BoxedAdapter = Box<dyn AgentAdapter>;

pub struct ProtocolAdapterRegistry {
    adapters: Vec<BoxedAdapter>,
}

impl ProtocolAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }
    
    pub fn register(&mut self, adapter: BoxedAdapter) {
        self.adapters.push(adapter);
    }
    
    pub fn get(&self, protocol: &AgentProtocol) -> Option<&dyn AgentAdapter> {
        self.adapters.iter().find(|a| a.protocol() == *protocol).map(|b| b.as_ref())
    }
    
    pub fn get_for_endpoint(&self, endpoint: &AgentEndpoint) -> Option<&dyn AgentAdapter> {
        self.get(&endpoint.protocol)
    }
    
    pub fn protocols(&self) -> Vec<AgentProtocol> {
        self.adapters.iter().map(|a| a.protocol()).collect()
    }
    
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(super::a2a_adapter::A2AAdapter::new()));
        registry.register(Box::new(super::opencode_adapter::OpenCodeAdapter::new()));
        registry.register(Box::new(super::openglow_adapter::OpenGlowAdapter::new()));
        registry
    }
}

impl Default for ProtocolAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
