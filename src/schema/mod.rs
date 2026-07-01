use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: RoleType,
    pub content: Content,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleType {
    System,
    User,
    Assistant,
    Tool,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleType::System => "system",
            RoleType::User => "user",
            RoleType::Assistant => "assistant",
            RoleType::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Image { url: String },
    Audio { url: String },
    Video { url: String },
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub part_type: String,
    pub text: Option<String>,
    pub image_url: Option<ImageUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArgument(pub HashMap<String, serde_json::Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: ToolResultContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Image { url: String },
    Audio { url: String },
    Video { url: String },
    Error(String),
}

impl Message {
    pub fn user(content: impl Into<Content>) -> Self {
        Self {
            role: RoleType::User,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<Content>) -> Self {
        Self {
            role: RoleType::Assistant,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn system(content: impl Into<Content>) -> Self {
        Self {
            role: RoleType::System,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool(content: String, tool_call_id: String) -> Self {
        Self {
            role: RoleType::Tool,
            content: Content::Text(content),
            name: None,
            tool_call_id: Some(tool_call_id),
            tool_calls: None,
        }
    }

    /// Convert to the a2a-rs SDK's Message type.
    pub fn into_a2a_message(self) -> a2a::Message {
        let role = match self.role {
            RoleType::User => a2a::Role::User,
            RoleType::Assistant => a2a::Role::Agent,
            RoleType::System => a2a::Role::User, // SDK doesn't have System
            RoleType::Tool => a2a::Role::User,   // SDK doesn't have Tool
        };

        let parts = match self.content {
            Content::Text(text) => vec![a2a::Part::text(text)],
            _ => vec![], // TODO: map other content types
        };

        a2a::Message {
            message_id: format!("msg-{}", uuid::Uuid::new_v4()),
            context_id: None,
            task_id: None,
            role,
            parts,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }
    }

    /// Convert from the a2a-rs SDK's Message type.
    pub fn from_a2a_message(msg: a2a::Message) -> Self {
        let role = match msg.role {
            a2a::Role::User => RoleType::User,
            a2a::Role::Agent => RoleType::Assistant,
            a2a::Role::Unspecified => RoleType::User,
        };

        let content = msg
            .parts
            .into_iter()
            .filter_map(|p| match p.content {
                a2a::PartContent::Text(text) => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        Self {
            role,
            content: Content::Text(content),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}
