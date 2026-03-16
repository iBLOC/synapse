use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{LLMProvider, LLMError, LLMResult, Message, Content, ChatOptions, Stream};

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4o".to_string(),
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
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| LLMError::Api("OPENAI_API_KEY not set".to_string()))?;
        
        Ok(Self::new(api_key))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn provider_name(&self) -> &str {
        "openai"
    }
    
    fn model(&self) -> &str {
        &self.default_model
    }
    
    async fn chat(
        &self,
        messages: Vec<Message>,
        options: ChatOptions,
    ) -> LLMResult<Message> {
        let model = options.model.unwrap_or_else(|| self.default_model.clone());
        
        let request = OpenAIChatRequest {
            model: model.clone(),
            messages: messages.iter().map(|m| OpenAIMessage {
                role: match m.role {
                    super::Role::System => "system".to_string(),
                    super::Role::User => "user".to_string(),
                    super::Role::Assistant => "assistant".to_string(),
                    super::Role::Tool => "tool".to_string(),
                },
                content: m.content.text.clone().unwrap_or_default(),
            }).collect(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stop: options.stop,
            tools: options.tools.map(|tools| {
                tools.into_iter().map(|t| serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })).collect()
            }),
            tool_choice: options.tool_choice.map(|c| match c {
                super::ToolChoice::Auto => serde_json::json!("auto"),
                super::ToolChoice::None => serde_json::json!("none"),
                super::ToolChoice::Function { name } => serde_json::json!({
                    "type": "function",
                    "function": { "name": name }
                }),
            }),
        };
        
        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        
        let response: OpenAIChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;
        
        let content = response.choices.first()
            .ok_or_else(|| LLMError::Api("No choices in response".to_string()))?
            .message.content.clone();
        
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
        Err(LLMError::StreamError("Streaming not implemented".to_string()))
    }
    
    async fn embeddings(&self, texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>> {
        let request = serde_json::json!({
            "input": texts,
            "model": "text-embedding-3-small",
        });
        
        let response = self.client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        
        let response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Api(e.to_string()))?;
        
        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }
}

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    tools: Option<Vec<serde_json::Value>>,
    tool_choice: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}
