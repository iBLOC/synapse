use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};

use super::{ChatOptions, Content, LLMError, LLMProvider, LLMResult, Message};

pub struct AzureProvider {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
    deployment: String,
}

impl AzureProvider {
    pub fn new(api_key: String, endpoint: String, deployment: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            endpoint,
            deployment,
        }
    }

    pub fn from_env() -> LLMResult<Self> {
        let api_key = std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| LLMError::Api("AZURE_OPENAI_API_KEY not set".to_string()))?;

        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
            .map_err(|_| LLMError::Api("AZURE_OPENAI_ENDPOINT not set".to_string()))?;

        let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT")
            .map_err(|_| LLMError::Api("AZURE_OPENAI_DEPLOYMENT not set".to_string()))?;

        Ok(Self::new(api_key, endpoint, deployment))
    }
}

#[async_trait]
impl LLMProvider for AzureProvider {
    fn provider_name(&self) -> &str {
        "azure"
    }

    fn model(&self) -> &str {
        &self.deployment
    }

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-02-15-preview",
            self.endpoint, self.deployment
        );

        let request = AzureChatRequest {
            messages: messages
                .iter()
                .map(|m| AzureMessage {
                    role: match m.role {
                        super::Role::System => "system",
                        super::Role::User => "user",
                        super::Role::Assistant => "assistant",
                        super::Role::Tool => "tool",
                    },
                    content: m.content.text.as_deref().unwrap_or(""),
                })
                .collect(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
        };

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
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

        let response: AzureChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;

        let content = response
            .choices
            .first()
            .ok_or_else(|| LLMError::Api("No choices in response".to_string()))?
            .message
            .content
            .clone();

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

    async fn embeddings(&self, texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>> {
        let url = format!(
            "{}/openai/deployments/{}/embeddings?api-version=2023-05-15",
            self.endpoint, self.deployment
        );

        let request = serde_json::json!({
            "input": texts,
        });

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
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

        let response: AzureEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;

        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }
}

#[derive(Serialize)]
struct AzureChatRequest<'a> {
    messages: Vec<AzureMessage<'a>>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
}

#[derive(Serialize)]
struct AzureMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct AzureChatResponse {
    choices: Vec<AzureChoice>,
}

#[derive(Deserialize)]
struct AzureChoice {
    message: AzureMessageResponse,
}

#[derive(Deserialize)]
struct AzureMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct AzureEmbeddingResponse {
    data: Vec<AzureEmbedding>,
}

#[derive(Deserialize)]
struct AzureEmbedding {
    embedding: Vec<f32>,
}
