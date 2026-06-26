use async_trait::async_trait;
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatOptions, Content, LLMError, LLMProvider, LLMResult, Message};

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub fn from_env() -> LLMResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::Api("ANTHROPIC_API_KEY not set".to_string()))?;

        Ok(Self::new(api_key))
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message> {
        let model = options.model.as_deref().unwrap_or(&self.default_model);

        let system = messages
            .iter()
            .find(|m| m.role == super::Role::System)
            .and_then(|m| m.content.text.clone());

        let user_messages: Vec<&Message> = messages
            .iter()
            .filter(|m| m.role != super::Role::System)
            .collect();

        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens: options.max_tokens.unwrap_or(4096) as i32,
            system,
            messages: user_messages
                .into_iter()
                .map(|m| AnthropicMessage {
                    role: match m.role {
                        super::Role::User => "user".to_string(),
                        super::Role::Assistant => "assistant".to_string(),
                        _ => "user".to_string(),
                    },
                    content: m.content.text.clone().unwrap_or_default(),
                })
                .collect(),
            temperature: options.temperature,
        };

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!("{} - {}", status, body)));
        }

        let response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;

        let content = response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(Message {
            role: super::Role::Assistant,
            content: Content::text(content),
        })
    }

    async fn stream_chat(
        &self,
        _messages: Vec<Message>,
        _options: ChatOptions,
    ) -> LLMResult<Box<dyn Stream<Item = LLMResult<Message>> + Send>> {
        Err(LLMError::StreamError(
            "Streaming not implemented".to_string(),
        ))
    }

    async fn embeddings(&self, _texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>> {
        Err(LLMError::Api(
            "Embeddings not supported for Anthropic".to_string(),
        ))
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    temperature: Option<f32>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}
