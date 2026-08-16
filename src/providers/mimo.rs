use async_trait::async_trait;

use super::openai::OpenAIProvider;
use super::{ChatOptions, LLMError, LLMProvider, LLMResult, Message};

pub struct MimoProvider {
    inner: OpenAIProvider,
}

impl MimoProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key)
                .with_base_url("https://api.xiaomimimo.com/v1")
                .with_model("mimo-v2.5-pro"),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.with_model(model);
        self
    }

    pub fn from_env() -> LLMResult<Self> {
        let api_key = std::env::var("MIMO_API_KEY")
            .or_else(|_| std::env::var("LLM_API_KEY"))
            .map_err(|_| LLMError::Api("MIMO_API_KEY or LLM_API_KEY not set".to_string()))?;

        Ok(Self::new(api_key))
    }
}

#[async_trait]
impl LLMProvider for MimoProvider {
    fn provider_name(&self) -> &str {
        "mimo"
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message> {
        self.inner.chat(messages, options).await
    }

    async fn stream_chat(
        &self,
        messages: Vec<Message>,
        options: ChatOptions,
    ) -> LLMResult<Box<dyn futures::Stream<Item = super::LLMResult<Message>> + Send>> {
        self.inner.stream_chat(messages, options).await
    }

    async fn embeddings(&self, texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>> {
        self.inner.embeddings(texts).await
    }
}
