use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::protocol::{
    AdapterResult, AgentAdapter, AgentEndpoint, AgentProtocol, AgentRequest, AgentResponse,
    ProtocolMessage, TaskStatus, TokenUsage,
};

pub struct OpenGlowAdapter {
    client: Client,
}

impl OpenGlowAdapter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn register(registry: &mut crate::protocol::ProtocolAdapterRegistry) {
        registry.register(Box::new(Self::new()));
    }
}

impl Default for OpenGlowAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentAdapter for OpenGlowAdapter {
    fn protocol(&self) -> AgentProtocol {
        AgentProtocol::OpenGlow
    }

    async fn send_task(
        &self,
        endpoint: &AgentEndpoint,
        request: &AgentRequest,
    ) -> AdapterResult<AgentResponse> {
        let openglow_request = json!({
            "action": "execute",
            "task_id": request.task_id,
            "prompt": request.messages.last().map(|m| m.content.as_str()).unwrap_or(""),
            "messages": request.messages,
            "context": request.context
        });

        let response = self
            .client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Protocol-Version", "1.0")
            .json(&openglow_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        let status = match result["state"].as_str().unwrap_or("pending") {
            "done" => TaskStatus::Completed,
            "error" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };

        let response_content = result["response"].as_str().unwrap_or("");

        let messages = vec![ProtocolMessage {
            role: "assistant".to_string(),
            content: response_content.to_string(),
            metadata: serde_json::Value::Null,
        }];

        let usage = TokenUsage {
            prompt_tokens: result["tokens"]["input"].as_u64().unwrap_or(0) as u32,
            completion_tokens: result["tokens"]["output"].as_u64().unwrap_or(0) as u32,
            total_tokens: result["tokens"]["total"].as_u64().unwrap_or(0) as u32,
            cost: result["cost"].as_f64().unwrap_or(0.0),
        };

        Ok(AgentResponse {
            task_id: request.task_id.clone(),
            status,
            messages,
            result: result["data"].clone(),
            usage,
        })
    }

    async fn get_task_status(
        &self,
        endpoint: &AgentEndpoint,
        task_id: &str,
    ) -> AdapterResult<AgentResponse> {
        let status_request = json!({
            "action": "status",
            "task_id": task_id
        });

        let response = self
            .client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Protocol-Version", "1.0")
            .json(&status_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        let status = match result["state"].as_str().unwrap_or("pending") {
            "done" => TaskStatus::Completed,
            "error" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "running" => TaskStatus::Running,
            _ => TaskStatus::Pending,
        };

        Ok(AgentResponse {
            task_id: task_id.to_string(),
            status,
            messages: vec![],
            result: result["data"].clone(),
            usage: TokenUsage::default(),
        })
    }

    async fn cancel_task(&self, endpoint: &AgentEndpoint, task_id: &str) -> AdapterResult<()> {
        let cancel_request = json!({
            "action": "cancel",
            "task_id": task_id
        });

        self.client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Protocol-Version", "1.0")
            .json(&cancel_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn discover_agent(&self, url: &str) -> AdapterResult<AgentEndpoint> {
        let info_request = json!({
            "action": "info"
        });

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Protocol-Version", "1.0")
            .json(&info_request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let info: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        let capabilities = info["abilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(AgentEndpoint {
            url: url.to_string(),
            protocol: AgentProtocol::OpenGlow,
            capabilities,
            metadata: info,
        })
    }
}
