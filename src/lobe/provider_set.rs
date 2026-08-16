//! `ProviderSet` — bundles TTS/ASR/LLM/Emotion providers for a lobe.

use std::sync::Arc;

use crate::providers::{ChatOptions, Content, ContentPart, LLMError, LLMProvider, Message, Role};

pub struct ProviderSet {
    pub llm: Arc<dyn LLMProvider>,
}

impl ProviderSet {
    pub fn from_env() -> Result<Self, String> {
        let llm_url = std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8085".into());
        let llm_model = std::env::var("LLM_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".into());
        let llm: Arc<dyn LLMProvider> = Arc::new(OpenAILlm::new(llm_url, llm_model));
        Ok(Self { llm })
    }
}

/// Minimal OpenAI-compatible LLM client.
pub struct OpenAILlm {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

impl OpenAILlm {
    pub fn new(base_url: String, model: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest build");
        Self { base_url, model, http }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAILlm {
    fn provider_name(&self) -> &str { "openai" }
    fn model(&self) -> &str { &self.model }

    async fn chat(
        &self,
        messages: Vec<Message>,
        options: ChatOptions,
    ) -> Result<crate::providers::Message, LLMError> {
        let model = options.model.unwrap_or_else(|| self.model.clone());
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": format!("{:?}", m.role).to_lowercase(),
                "content": m.content.parts.iter().filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.clone()),
                    _ => None,
                }).collect::<Vec<_>>().join("\n")
            })).collect::<Vec<_>>(),
            "temperature": options.temperature,
            "max_tokens": options.max_tokens,
        });
        let resp = self.http.post(&url).json(&body).send().await
            .map_err(|e| LLMError::Api(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(LLMError::Api(format!("HTTP {}", resp.status())));
        }
        let v: serde_json::Value = resp.json().await
            .map_err(|e| LLMError::Api(e.to_string()))?;
        let content = v["choices"][0]["message"]["content"].as_str()
            .unwrap_or("")
            .to_string();
        Ok(crate::providers::Message {
            role: Role::Assistant,
            content: Content::text(content),
        })
    }

    async fn stream_chat(
        &self,
        _messages: Vec<Message>,
        _options: ChatOptions,
    ) -> Result<Box<dyn futures::stream::Stream<Item = Result<Message, LLMError>> + Send>, LLMError> {
        Err(LLMError::StreamError("not implemented".into()))
    }

    async fn embeddings(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, LLMError> {
        Err(LLMError::Api("not implemented".into()))
    }
}
