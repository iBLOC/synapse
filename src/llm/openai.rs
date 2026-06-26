use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;
use crate::transport::HttpClient;
use crate::components::model::{BaseChatModel, GenerateOptions, ModelResult, ModelError};
use crate::schema::Message;

pub struct OpenAIClient {
    http: Arc<dyn HttpClient>,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(http: Arc<dyn HttpClient>, api_key: impl Into<String>) -> Self {
        Self {
            http,
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn build_request_body(&self, messages: &[Message], options: &GenerateOptions) -> serde_json::Value {
        json!({
            "model": options.model.clone().unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            "messages": messages.iter().map(|m| json!({
                "role": m.role.as_str(),
                "content": match &m.content {
                    crate::schema::Content::Text(s) => s.clone(),
                    _ => String::new(),
                }
            })).collect::<Vec<_>>(),
        })
    }
}

#[async_trait]
impl BaseChatModel for OpenAIClient {
    async fn generate(&self, messages: Vec<Message>, options: GenerateOptions) -> ModelResult<Message> {
        let body = self.build_request_body(&messages, &options);
        let url = format!("{}/chat/completions", self.base_url);

        let resp: serde_json::Value = self.http.post_json_with_headers(
            &url,
            &[("Authorization", &format!("Bearer {}", self.api_key))],
            &body,
        ).await.map_err(|e| ModelError::Api(e.to_string()))?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ModelError::Api("Invalid OpenAI response format".to_string()))?;

        Ok(Message::assistant(content))
    }

    async fn stream(&self, _messages: Vec<Message>, _options: GenerateOptions) -> ModelResult<crate::components::model::StreamReader> {
        Err(ModelError::Api("OpenAIClient::stream not yet implemented".to_string()))
    }
}
