use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub type LLMResult<T> = Result<T, LLMError>;

pub type LLMStream = Pin<Box<dyn Stream<Item = LLMResult<Message>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMError {
    Api(String),
    InvalidRequest(String),
    RateLimit(String),
    Timeout,
    ContextLengthExceeded,
    StreamError(String),
}

impl std::fmt::Display for LLMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMError::Api(msg) => write!(f, "API error: {}", msg),
            LLMError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LLMError::RateLimit(msg) => write!(f, "Rate limit: {}", msg),
            LLMError::Timeout => write!(f, "Timeout"),
            LLMError::ContextLengthExceeded => write!(f, "Context length exceeded"),
            LLMError::StreamError(msg) => write!(f, "Stream error: {}", msg),
        }
    }
}

impl std::error::Error for LLMError {}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn model(&self) -> &str;

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message>;

    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        options: ChatOptions,
    ) -> LLMResult<Box<dyn Stream<Item = LLMResult<Message>> + Send>>;

    async fn embeddings(&self, texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub text: Option<String>,
    #[serde(default)]
    pub parts: Vec<ContentPart>,
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            parts: vec![],
        }
    }

    pub fn from_parts(parts: Vec<ContentPart>) -> Self {
        Self { text: None, parts }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::text(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        url: String,
    },
    ToolUse {
        id: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        result: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
    None,
    Function { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    Text,
    Json,
}

#[cfg(not(target_arch = "wasm32"))]
pub mod anthropic;
#[cfg(not(target_arch = "wasm32"))]
pub mod azure;
#[cfg(not(target_arch = "wasm32"))]
pub mod openai;
