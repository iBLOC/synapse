use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerKind {
    Dm,
    Group,
    Channel,
    Thread,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerInfo {
    pub kind: PeerKind,
    pub id: String,
    pub display_name: Option<String>,
}

impl PeerInfo {
    pub fn dm(id: impl Into<String>) -> Self {
        Self {
            kind: PeerKind::Dm,
            id: id.into(),
            display_name: None,
        }
    }

    pub fn group(id: impl Into<String>) -> Self {
        Self {
            kind: PeerKind::Group,
            id: id.into(),
            display_name: None,
        }
    }

    pub fn channel(id: impl Into<String>) -> Self {
        Self {
            kind: PeerKind::Channel,
            id: id.into(),
            display_name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelTarget {
    pub channel_id: ChannelId,
    pub account_id: Option<String>,
    pub peer: PeerInfo,
}

impl ChannelTarget {
    pub fn new(channel_id: impl Into<String>, peer: PeerInfo) -> Self {
        Self {
            channel_id: ChannelId::new(channel_id),
            account_id: None,
            peer,
        }
    }

    pub fn with_account(mut self, account_id: impl Into<String>) -> Self {
        self.account_id = Some(account_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub account_id: Option<String>,
    pub from: PeerInfo,
    pub to: ChannelTarget,
    pub text: String,
    pub reply_to: Option<MessageId>,
    pub thread_root: Option<MessageId>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ChannelMessage {
    pub fn new(
        channel_id: impl Into<String>,
        from: PeerInfo,
        to: ChannelTarget,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: MessageId::new(uuid::Uuid::new_v4().to_string()),
            channel_id: ChannelId::new(channel_id),
            account_id: None,
            from,
            to,
            text: text.into(),
            reply_to: None,
            thread_root: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn is_dm(&self) -> bool {
        matches!(self.to.peer.kind, PeerKind::Dm)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResponse {
    pub text: String,
    pub reply_to: Option<MessageId>,
    pub thread_root: Option<MessageId>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl ChannelResponse {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_to: None,
            thread_root: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn reply_to(mut self, id: MessageId) -> Self {
        self.reply_to = Some(id);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_id() {
        let id = ChannelId::new("telegram");
        assert_eq!(id.as_str(), "telegram");
        assert_eq!(id.to_string(), "telegram");
    }

    #[test]
    fn test_peer_info() {
        let peer = PeerInfo::dm("user123").with_name("Alice");
        assert_eq!(peer.kind, PeerKind::Dm);
        assert_eq!(peer.id, "user123");
        assert_eq!(peer.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_channel_message_creation() {
        let from = PeerInfo::dm("sender");
        let to = ChannelTarget::new("slack", PeerInfo::group("general"));
        let msg = ChannelMessage::new("slack", from, to, "Hello, world!");
        assert_eq!(msg.text, "Hello, world!");
        assert!(!msg.is_dm());
    }

    #[test]
    fn test_channel_response() {
        let resp = ChannelResponse::new("Hi back!")
            .reply_to(MessageId::new("msg-123"))
            .with_metadata("priority", serde_json::json!("high"));
        assert_eq!(resp.text, "Hi back!");
        assert!(resp.reply_to.is_some());
        assert_eq!(resp.metadata.get("priority").unwrap(), "high");
    }
}