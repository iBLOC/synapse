use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::protocol::{AgentAdapter, AgentEndpoint, AgentProtocol, AgentRequest, AgentResponse, ProtocolMessage, ProtocolAdapterRegistry, TaskStatus, TokenUsage, AdapterResult};

pub struct A2AAdapter {
    client: Client,
}

impl A2AAdapter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub fn register(registry: &mut ProtocolAdapterRegistry) {
        registry.register(Box::new(Self::new()));
    }
}

impl Default for A2AAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentAdapter for A2AAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::A2A
    }
    
    async fn send_task(&self, endpoint: &AgentEndpoint, request: &AgentRequest) -> AdapterResult<AgentResponse> {
        let message = json!({
            "role": request.messages.last().map(|m| m.role.as_str()).unwrap_or("user"),
            "parts": [{
                "type": "text",
                "text": request.messages.last().map(|m| m.content.as_str()).unwrap_or("")
            }]
        });
        
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/send",
            "params": {
                "id": request.task_id,
                "message": message
            }
        });
        
        let response = self.client
            .post(&endpoint.url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let rpc_response: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        if let Some(error) = rpc_response.get("error") {
            return Err(format!("A2A error: {}", error));
        }
        
        let task = rpc_response["result"].clone();
        
        let status = match task["status"].as_str().unwrap_or("pending") {
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "canceled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };
        
        let messages: Vec<ProtocolMessage> = task["messages"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|m| {
                    Some(ProtocolMessage {
                        role: m["role"].as_str()?.to_string(),
                        content: m["parts"].as_array()?.first()?.get("text")?.as_str()?.to_string(),
                        metadata: serde_json::Value::Null,
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        let result = task["result"].clone();
        
        Ok(AgentResponse {
            task_id: request.task_id.clone(),
            status,
            messages,
            result,
            usage: TokenUsage::default(),
        })
    }
    
    async fn get_task_status(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<AgentResponse> {
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/get",
            "params": {
                "id": task_id
            }
        });
        
        let response = self.client
            .post(&endpoint.url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let rpc_response: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let task = rpc_response["result"].clone();
        
        let status = match task["status"].as_str().unwrap_or("pending") {
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "canceled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };
        
        Ok(AgentResponse {
            task_id: task_id.to_string(),
            status,
            messages: vec![],
            result: task.get("result").cloned().unwrap_or(serde_json::Value::Null),
            usage: TokenUsage::default(),
        })
    }
    
    async fn cancel_task(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<()> {
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tasks/cancel",
            "params": {
                "id": task_id
            }
        });
        
        self.client
            .post(&endpoint.url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    async fn discover_agent(&self, url: &str) -> AdapterResult<AgentEndpoint> {
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "agents/get",
            "params": {
                "url": url
            }
        });
        
        let response = self.client
            .post(url)
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let card: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        let capabilities = card["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(AgentEndpoint {
            url: url.to_string(),
            protocol: AgentProtocol::A2A,
            capabilities,
            metadata: card,
        })
    }
}
