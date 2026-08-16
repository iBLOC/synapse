use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::message::{ChannelMessage, ChannelResponse, ChannelTarget};

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Message send error: {0}")]
    Send(String),

    #[error("Message receive error: {0}")]
    Receive(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Channel not available: {0}")]
    Unavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ChannelResult<T> = Result<T, ChannelError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMeta {
    pub id: String,
    pub label: String,
    pub selection_label: String,
    pub docs_path: Option<String>,
    pub docs_label: Option<String>,
    pub blurb: String,
    pub order: i32,
}

impl ChannelMeta {
    pub fn new(id: impl Into<String>, label: impl Into<String>, blurb: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            label: label.into(),
            selection_label: id_str,
            docs_path: None,
            docs_label: None,
            blurb: blurb.into(),
            order: 100,
        }
    }

    pub fn with_selection_label(mut self, label: impl Into<String>) -> Self {
        self.selection_label = label.into();
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    pub supports_dm: bool,
    pub supports_group: bool,
    pub supports_thread: bool,
    pub supports_reactions: bool,
    pub supports_media: bool,
    pub supports_polls: bool,
    pub supports_typing: bool,
    pub supports_read_receipts: bool,
}

#[async_trait]
pub trait ConfigAdapter: Send + Sync {
    async fn validate(&self, config: &serde_json::Value) -> ChannelResult<()>;
    fn schema(&self) -> serde_json::Value;
}

#[async_trait]
pub trait GatewayAdapter: Send + Sync {
    async fn start(
        &self,
        account: serde_json::Value,
        on_message: std::sync::Arc<dyn Fn(ChannelMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = ChannelResult<ChannelResponse>> + Send>> + Send + Sync>,
    ) -> ChannelResult<Box<dyn std::any::Any + Send + Sync>>;

    async fn stop(&self, handle: Box<dyn std::any::Any + Send + Sync>) -> ChannelResult<()>;
}

#[async_trait]
pub trait OutboundAdapter: Send + Sync {
    async fn send(
        &self,
        account: &serde_json::Value,
        target: &ChannelTarget,
        response: &ChannelResponse,
    ) -> ChannelResult<()>;

    async fn send_typing(&self, account: &serde_json::Value, target: &ChannelTarget) -> ChannelResult<()> {
        let _ = (account, target);
        Ok(())
    }
}

#[async_trait]
pub trait StatusAdapter: Send + Sync {
    async fn status(&self, account: &serde_json::Value) -> ChannelResult<ChannelStatus>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub connected: bool,
    pub account_id: Option<String>,
    pub details: std::collections::HashMap<String, serde_json::Value>,
}

impl ChannelStatus {
    pub fn connected() -> Self {
        Self {
            connected: true,
            account_id: None,
            details: std::collections::HashMap::new(),
        }
    }

    pub fn disconnected() -> Self {
        Self {
            connected: false,
            account_id: None,
            details: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
pub trait ChannelPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn meta(&self) -> &ChannelMeta;
    fn capabilities(&self) -> &ChannelCapabilities;

    fn config(&self) -> Option<&dyn ConfigAdapter> {
        None
    }

    fn gateway(&self) -> Option<&dyn GatewayAdapter> {
        None
    }

    fn outbound(&self) -> Option<&dyn OutboundAdapter> {
        None
    }

    fn status(&self) -> Option<&dyn StatusAdapter> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_meta() {
        let meta = ChannelMeta::new("telegram", "Telegram", "Telegram Bot API");
        assert_eq!(meta.id, "telegram");
        assert_eq!(meta.label, "Telegram");
    }

    #[test]
    fn test_capabilities_default() {
        let caps = ChannelCapabilities::default();
        assert!(!caps.supports_dm);
        assert!(!caps.supports_group);
    }

    #[test]
    fn test_channel_status() {
        let connected = ChannelStatus::connected();
        assert!(connected.connected);

        let disconnected = ChannelStatus::disconnected();
        assert!(!disconnected.connected);
    }
}