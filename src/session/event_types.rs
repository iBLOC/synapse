use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub event_type: SessionEventType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    UserMessage,
    AssistantMessage,
    AssistantChunk,
    ToolCall,
    ToolResult,
    StepStart,
    StepEnd,
    TurnStart,
    TurnEnd,
    SystemMessage,
    Custom(String),
}

impl SessionEvent {
    pub fn new(
        session_id: Uuid,
        event_type: SessionEventType,
        payload: serde_json::Value,
        sequence: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            event_type,
            payload,
            timestamp: Utc::now(),
            sequence,
        }
    }

    pub fn user_message(session_id: Uuid, content: &str, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::UserMessage,
            serde_json::json!({ "content": content }),
            sequence,
        )
    }

    pub fn assistant_message(session_id: Uuid, content: &str, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::AssistantMessage,
            serde_json::json!({ "content": content }),
            sequence,
        )
    }

    pub fn assistant_chunk(session_id: Uuid, delta: &str, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::AssistantChunk,
            serde_json::json!({ "delta": delta }),
            sequence,
        )
    }

    pub fn tool_call(session_id: Uuid, tool_name: &str, args: &serde_json::Value, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::ToolCall,
            serde_json::json!({ "tool": tool_name, "arguments": args }),
            sequence,
        )
    }

    pub fn tool_result(session_id: Uuid, tool_name: &str, result: &serde_json::Value, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::ToolResult,
            serde_json::json!({ "tool": tool_name, "result": result }),
            sequence,
        )
    }

    pub fn step_start(session_id: Uuid, step_id: &str, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::StepStart,
            serde_json::json!({ "step_id": step_id }),
            sequence,
        )
    }

    pub fn step_end(session_id: Uuid, step_id: &str, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::StepEnd,
            serde_json::json!({ "step_id": step_id }),
            sequence,
        )
    }

    pub fn turn_start(session_id: Uuid, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::TurnStart,
            serde_json::json!({}),
            sequence,
        )
    }

    pub fn turn_end(session_id: Uuid, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::TurnEnd,
            serde_json::json!({}),
            sequence,
        )
    }

    pub fn custom(session_id: Uuid, event_name: &str, data: serde_json::Value, sequence: u64) -> Self {
        Self::new(
            session_id,
            SessionEventType::Custom(event_name.to_string()),
            data,
            sequence,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    pub version: u32,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl SessionHeader {
    pub fn new(id: Uuid) -> Self {
        Self {
            version: 1,
            id,
            created_at: Utc::now(),
            name: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
