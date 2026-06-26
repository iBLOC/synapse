use std::sync::Arc;
use async_trait::async_trait;
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
}

#[async_trait]
impl BaseChatModel for OpenAIClient {
    async fn generate(&self, _messages: Vec<Message>, _options: GenerateOptions) -> ModelResult<Message> {
        Err(ModelError::Api("OpenAIClient::generate not yet implemented".to_string()))
    }

    async fn stream(&self, _messages: Vec<Message>, _options: GenerateOptions) -> ModelResult<crate::components::model::StreamReader> {
        Err(ModelError::Api("OpenAIClient::stream not yet implemented".to_string()))
    }
}
