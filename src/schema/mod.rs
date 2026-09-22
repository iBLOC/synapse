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
            Content::Image { url } => {
                vec![a2a::Part::url(url).with_media_type("image/png")]
            }
            Content::Audio { url } => {
                vec![a2a::Part::url(url).with_media_type("audio/wav")]
            }
            Content::Video { url } => {
                vec![a2a::Part::url(url).with_media_type("video/mp4")]
            }
            Content::Parts(content_parts) => content_parts
                .into_iter()
                .map(|p| content_part_to_a2a_part(p))
                .collect(),
        };

        // encephalon packs v0.1 metadata into the v0.2 'name' field with
        // the prefix __encephalon_meta_v1__ (see
        // a2a::client::METADATA_NAME_PREFIX). a2a-lf's Message has
        // no 'name' field, but it does carry a metadata map, so we
        // forward encephalon's packed name under the metadata["name"]
        // key. The reverse direction (from_a2a_message below) reads
        // the same key back into the v0.2 Message's name field.
        let metadata = self.name.as_ref().map(|n| {
            let mut m = std::collections::HashMap::new();
            m.insert("name".to_string(), serde_json::Value::String(n.clone()));
            m
        });

        a2a::Message {
            message_id: format!("msg-{}", uuid::Uuid::new_v4()),
            context_id: None,
            task_id: None,
            role,
            parts,
            metadata,
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
                a2a::PartContent::Url(url) => {
                    match p.media_type.as_deref() {
                        Some(m) if m.starts_with("image/") => {
                            Some(format!("[image: {}]", url))
                        }
                        Some(m) if m.starts_with("audio/") => {
                            Some(format!("[audio: {}]", url))
                        }
                        Some(m) if m.starts_with("video/") => {
                            Some(format!("[video: {}]", url))
                        }
                        _ => Some(url),
                    }
                }
                a2a::PartContent::Data(value) => {
                    Some(serde_json::to_string(&value).unwrap_or_default())
                }
                a2a::PartContent::Raw(_) => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Reverse of into_a2a_message: pull the encephalon-packed name
        // back out of metadata["name"] (a2a-lf's Message has no
        // 'name' field, but it carries a metadata map that we
        // overloaded in into_a2a_message).
        let name = msg
            .metadata
            .as_ref()
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Self {
            role,
            content: Content::Text(content),
            name,
            tool_call_id: None,
            tool_calls: None,
        }
    }
}

fn content_part_to_a2a_part(part: ContentPart) -> a2a::Part {
    match part.part_type.as_str() {
        "text" => {
            let text = part.text.unwrap_or_default();
            a2a::Part::text(text)
        }
        "image_url" => {
            let url = part
                .image_url
                .map(|iu| iu.url)
                .unwrap_or_default();
            a2a::Part::url(url).with_media_type("image/png")
        }
        _ => {
            let text = part.text.unwrap_or_default();
            a2a::Part::text(text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_into_a2a_user_role() {
        let m = Message::user("hi");
        let sdk_m = m.into_a2a_message();
        assert!(matches!(sdk_m.role, a2a::Role::User));
        assert_eq!(sdk_m.parts.len(), 1);
        assert!(matches!(&sdk_m.parts[0].content, a2a::PartContent::Text(t) if t == "hi"));
    }

    #[test]
    fn message_into_a2a_assistant_role_becomes_agent() {
        let m = Message::assistant("hello");
        let sdk_m = m.into_a2a_message();
        assert!(matches!(sdk_m.role, a2a::Role::Agent));
    }

    #[test]
    fn message_from_a2a_round_trips_text() {
        let sdk_m = a2a::Message {
            message_id: "m1".to_string(),
            context_id: None,
            task_id: None,
            role: a2a::Role::User,
            parts: vec![a2a::Part::text("hello world")],
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        };

        let schema_m = Message::from_a2a_message(sdk_m);
        assert_eq!(schema_m.role, RoleType::User);
        match schema_m.content {
            Content::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected Text content"),
        }
    }
}
