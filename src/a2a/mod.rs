pub mod card;
pub mod protocol;

#[cfg(not(target_arch = "wasm32"))]
use card::AgentCard;
#[cfg(not(target_arch = "wasm32"))]
use protocol::{
    AgentsGetParams, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Message, Task,
    TaskCancelParams, TaskGetParams, TaskSendParams, ERR_INTERNAL_ERROR,
};

#[cfg(not(target_arch = "wasm32"))]
pub type A2AResult<T> = Result<T, A2AError>;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum A2AError {
    #[error("network error: {0}")]
    Network(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("agent not found")]
    AgentNotFound,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct A2AClient {
    client: reqwest::Client,
    agent_url: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl A2AClient {
    pub fn new(agent_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            agent_url: agent_url.into(),
        }
    }

    pub async fn send_task(&self, task_id: &str, message: Message) -> A2AResult<Task> {
        let params = TaskSendParams {
            id: task_id.to_string(),
            message,
            push_notification: None,
        };

        let request =
            JsonRpcRequest::new("tasks/send", Some(serde_json::to_value(params).unwrap()));

        let response = self
            .client
            .post(&self.agent_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(A2AError::Protocol(error.message));
        }

        let task = serde_json::from_value(rpc_response.result.unwrap())
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        Ok(task)
    }

    pub async fn get_task(&self, task_id: &str) -> A2AResult<Task> {
        let params = TaskGetParams {
            id: task_id.to_string(),
        };

        let request = JsonRpcRequest::new("tasks/get", Some(serde_json::to_value(params).unwrap()));

        let response = self
            .client
            .post(&self.agent_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(A2AError::Protocol(error.message));
        }

        let task = serde_json::from_value(rpc_response.result.unwrap())
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        Ok(task)
    }

    pub async fn cancel_task(&self, task_id: &str) -> A2AResult<()> {
        let params = TaskCancelParams {
            id: task_id.to_string(),
        };

        let request =
            JsonRpcRequest::new("tasks/cancel", Some(serde_json::to_value(params).unwrap()));

        let response = self
            .client
            .post(&self.agent_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(A2AError::Protocol(error.message));
        }

        Ok(())
    }

    pub async fn get_agent_card(&self, url: Option<String>) -> A2AResult<AgentCard> {
        let target_url = url.unwrap_or_else(|| self.agent_url.clone());

        let params = AgentsGetParams {
            url: Some(target_url.clone()),
            metadata: None,
        };

        let request =
            JsonRpcRequest::new("agents/get", Some(serde_json::to_value(params).unwrap()));

        let response = self
            .client
            .post(&target_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;

        let rpc_response: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        if let Some(error) = rpc_response.error {
            return Err(A2AError::Protocol(error.message));
        }

        let card = serde_json::from_value(rpc_response.result.unwrap())
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))?;

        Ok(card)
    }
}

use async_trait::async_trait;

#[async_trait]
#[cfg(not(target_arch = "wasm32"))]
pub trait TaskHandler: Send + Sync {
    async fn handle_task(&self, task_id: String, message: Message) -> A2AResult<Task>;
    fn get_task(&self, task_id: &str) -> A2AResult<Task>;
    fn cancel_task(&self, task_id: &str) -> A2AResult<()>;
}

#[cfg(not(target_arch = "wasm32"))]
pub struct A2AServer {
    card: AgentCard,
    task_handler: Box<dyn TaskHandler>,
}

#[cfg(not(target_arch = "wasm32"))]
impl A2AServer {
    pub fn new(card: AgentCard, handler: Box<dyn TaskHandler>) -> Self {
        Self {
            card,
            task_handler: handler,
        }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            "tasks/send" => {
                let params: Result<TaskSendParams, _> =
                    serde_json::from_value(request.params.unwrap_or_default());
                match params {
                    Ok(p) => match self.task_handler.handle_task(p.id, p.message).await {
                        Ok(task) => {
                            JsonRpcResponse::success(id, serde_json::to_value(task).unwrap())
                        }
                        Err(e) => JsonRpcResponse::error(
                            id,
                            JsonRpcError::new(ERR_INTERNAL_ERROR, e.to_string()),
                        ),
                    },
                    Err(e) => JsonRpcResponse::error(id, JsonRpcError::new(-32602, e.to_string())),
                }
            }
            "tasks/get" => {
                let params: Result<TaskGetParams, _> =
                    serde_json::from_value(request.params.unwrap_or_default());
                match params {
                    Ok(p) => match self.task_handler.get_task(&p.id) {
                        Ok(task) => {
                            JsonRpcResponse::success(id, serde_json::to_value(task).unwrap())
                        }
                        Err(e) => JsonRpcResponse::error(
                            id,
                            JsonRpcError::new(ERR_INTERNAL_ERROR, e.to_string()),
                        ),
                    },
                    Err(e) => JsonRpcResponse::error(id, JsonRpcError::new(-32602, e.to_string())),
                }
            }
            "tasks/cancel" => {
                let params: Result<TaskCancelParams, _> =
                    serde_json::from_value(request.params.unwrap_or_default());
                match params {
                    Ok(p) => match self.task_handler.cancel_task(&p.id) {
                        Ok(_) => JsonRpcResponse::success(id, serde_json::json!({ "ok": true })),
                        Err(e) => JsonRpcResponse::error(
                            id,
                            JsonRpcError::new(ERR_INTERNAL_ERROR, e.to_string()),
                        ),
                    },
                    Err(e) => JsonRpcResponse::error(id, JsonRpcError::new(-32602, e.to_string())),
                }
            }
            "agents/get" => JsonRpcResponse::success(id, serde_json::to_value(&self.card).unwrap()),
            _ => JsonRpcResponse::error(id, JsonRpcError::new(-32601, "Method not found")),
        }
    }

    pub fn agent_card(&self) -> &AgentCard {
        &self.card
    }
}
