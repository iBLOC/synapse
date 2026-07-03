//! Backwards-compatibility shim for the v0.1 A2A surface.
//!
//! v0.2 of `synapse::a2a` adopts the official `a2a-rs` SDK for wire
//! traffic, replacing the hand-rolled JSON-RPC stack that v0.1
//! exposed as `synapse::a2a::{A2AClient, A2AServer, TaskHandler,
//! A2AError, A2AResult}`.
//!
//! This module re-implements the v0.1 surface as a thin adapter on
//! top of the v0.2 SDK so downstream callers continue to compile
//! unchanged. Every public item here is `#[deprecated]` to nudge
//! callers toward the v0.2 wrappers (`a2a::client::A2AClient`,
//! `a2a::server::A2AServerBuilder`).

#![allow(deprecated)]

use async_trait::async_trait;

use crate::a2a::card::AgentCard;
use crate::a2a::protocol::{
    FilePart, Message, MessagePart, MessageRole, Task, TaskState, TaskStatus,
};

#[deprecated(
    since = "0.2.0",
    note = "use `a2a::A2AError` (re-exported from the SDK) instead"
)]
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

#[allow(deprecated)]
#[deprecated(note = "use `a2a::A2AError` instead")]
pub type A2AResult<T> = Result<T, A2AError>;

#[deprecated(note = "use `a2a::client::A2AClient` instead")]
pub struct A2AClient {
    client: reqwest::Client,
    agent_url: String,
}

#[allow(deprecated)]
impl A2AClient {
    #[allow(deprecated)]
    pub fn new(agent_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            agent_url: agent_url.into(),
        }
    }

    #[allow(deprecated)]
    pub async fn send_task(&self, task_id: &str, message: Message) -> A2AResult<Task> {
        let text = message
            .parts
            .iter()
            .filter_map(|p| match p {
                MessagePart::Text { text } => Some(text.clone()),
                MessagePart::File { file } => match &file.uri {
                    Some(u) => Some(u.clone()),
                    None => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        let schema_msg = crate::schema::Message {
            role: match message.role {
                MessageRole::User => crate::schema::RoleType::User,
                _ => crate::schema::RoleType::Assistant,
            },
            content: crate::schema::Content::Text(text),
            name: message
                .metadata
                .as_ref()
                .and_then(|m| m.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            tool_call_id: None,
            tool_calls: None,
        };
        let sdk = crate::a2a::client::A2AClient::new(&self.agent_url)
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;
        let reply = sdk
            .send_message(schema_msg)
            .await
            .map_err(|e| A2AError::Protocol(e.to_string()))?;
        let reply_text = match &reply.content {
            crate::schema::Content::Text(s) => s.clone(),
            _ => String::new(),
        };
        let now = chrono::Utc::now().timestamp_millis();
        Ok(Task {
            id: task_id.to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                message: Some(reply_text),
                timestamp: Some(now),
            },
            history: None,
            metadata: None,
        })
    }

    #[allow(deprecated)]
    pub async fn get_task(&self, _task_id: &str) -> A2AResult<Task> {
        Err(A2AError::TaskNotFound("get_task via v0.1 API".into()))
    }

    #[allow(deprecated)]
    pub async fn cancel_task(&self, _task_id: &str) -> A2AResult<()> {
        Ok(())
    }

    #[allow(deprecated)]
    pub async fn get_agent_card(&self, url: Option<String>) -> A2AResult<AgentCard> {
        let target = url.unwrap_or_else(|| self.agent_url.clone());
        let resp = self
            .client
            .get(&format!("{}/.well-known/agent-card.json", target))
            .send()
            .await
            .map_err(|e| A2AError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(A2AError::Protocol(format!("HTTP {}", resp.status())));
        }
        resp.json::<AgentCard>()
            .await
            .map_err(|e| A2AError::InvalidResponse(e.to_string()))
    }
}

#[async_trait]
#[deprecated(note = "use `a2a::server::A2AServerBuilder` instead")]
pub trait TaskHandler: Send + Sync {
    async fn handle_task(&self, task_id: String, message: Message) -> A2AResult<Task>;
    fn get_task(&self, task_id: &str) -> A2AResult<Task>;
    fn cancel_task(&self, task_id: &str) -> A2AResult<()>;
}

#[deprecated(note = "use `a2a::server::A2AServerBuilder` instead")]
pub struct A2AServer {
    card: AgentCard,
    task_handler: Box<dyn TaskHandler>,
}

#[allow(deprecated)]
impl A2AServer {
    #[allow(deprecated)]
    pub fn new(card: AgentCard, handler: Box<dyn TaskHandler>) -> Self {
        Self {
            card,
            task_handler: handler,
        }
    }

    pub fn agent_card(&self) -> &AgentCard {
        &self.card
    }
}
