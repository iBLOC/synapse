use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::protocol::{AgentAdapter, AgentEndpoint, AgentProtocol, AgentRequest, AgentResponse, ProtocolMessage, TaskStatus, TokenUsage, AdapterResult};

pub struct OpenCodeAdapter {
    client: Client,
}

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub fn register(registry: &mut crate::protocol::ProtocolAdapterRegistry) {
        registry.register(Box::new(Self::new()));
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentAdapter for OpenCodeAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::OpenCode
    }
    
    async fn send_task(&self, endpoint: &AgentEndpoint, request: &AgentRequest) -> AdapterResult<AgentResponse> {
        let messages: Vec<serde_json::Value> = request.messages
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                    "metadata": m.metadata
                })
            })
            .collect();
        
        let opencode_request = json!({
            "type": "task",
            "command": "execute",
            "task_id": request.task_id,
            "messages": messages,
            "context": request.context
        });
        
        let response = self.client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .json(&opencode_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        let status = match result["status"].as_str().unwrap_or("pending") {
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };
        
        let response_messages: Vec<ProtocolMessage> = result["messages"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|m| ProtocolMessage {
                        role: m["role"].as_str().unwrap_or("assistant").to_string(),
                        content: m["content"].as_str().unwrap_or("").to_string(),
                        metadata: m["metadata"].clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        let usage = if let Some(u) = result.get("usage") {
            TokenUsage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                cost: u.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0),
            }
        } else {
            TokenUsage::default()
        };
        
        Ok(AgentResponse {
            task_id: request.task_id.clone(),
            status,
            messages: response_messages,
            result: result["result"].clone(),
            usage,
        })
    }
    
    async fn get_task_status(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<AgentResponse> {
        let status_request = json!({
            "type": "task",
            "command": "status",
            "task_id": task_id
        });
        
        let response = self.client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .json(&status_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        let status = match result["status"].as_str().unwrap_or("pending") {
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };
        
        Ok(AgentResponse {
            task_id: task_id.to_string(),
            status,
            messages: vec![],
            result: result["result"].clone(),
            usage: TokenUsage::default(),
        })
    }
    
    async fn cancel_task(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<()> {
        let cancel_request = json!({
            "type": "task",
            "command": "cancel",
            "task_id": task_id
        });
        
        self.client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .json(&cancel_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    async fn discover_agent(&self, url: &str) -> AdapterResult<AgentEndpoint> {
        let info_request = json!({
            "type": "agent",
            "command": "info"
        });
        
        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&info_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let info: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        let capabilities = info["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(AgentEndpoint {
            url: url.to_string(),
            protocol: AgentProtocol::OpenCode,
            capabilities,
            metadata: info,
        })
    }
}
