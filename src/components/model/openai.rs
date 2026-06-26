use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::components::model::{BaseChatModel, GenerateOptions, ModelError, StreamReader};
use crate::schema::Message;

pub struct OpenAIChatModel {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

impl OpenAIChatModel {
    pub fn new(config: OpenAIConfig) -> Self {
        let base_url = config.base_url.unwrap_or_else(|| "https://api.openai.com".to_string());
        
        Self {
            client: Client::new(),
            api_key: config.api_key,
            base_url,
            model: config.model,
        }
    }
    
    pub fn builder() -> OpenAIChatModelBuilder {
        OpenAIChatModelBuilder::new()
    }
    
    async fn build_request(&self, messages: Vec<crate::schema::Message>, options: GenerateOptions) -> serde_json::Value {
        let mut request = serde_json::json!({
            "model": options.model.as_ref().unwrap_or(&self.model),
            "messages": messages.iter().map(|m| self.message_to_openai(m)).collect::<Vec<_>>(),
        });
        
        if let Some(temp) = options.temperature {
            request["temperature"] = serde_json::json!(temp);
        }
        
        if let Some(max_tokens) = options.max_tokens {
            request["max_tokens"] = serde_json::json!(max_tokens);
        }
        
        if let Some(top_p) = options.top_p {
            request["top_p"] = serde_json::json!(top_p);
        }
        
        if let Some(tools) = options.tools {
            if !tools.is_empty() {
                request["tools"] = serde_json::json!(tools.iter().map(|t| serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })).collect::<Vec<_>>());
            }
        }
        
        request
    }
    
    fn message_to_openai(&self, msg: &crate::schema::Message) -> serde_json::Value {
        let role = match msg.role {
            crate::schema::RoleType::System => "system",
            crate::schema::RoleType::User => "user",
            crate::schema::RoleType::Assistant => "assistant",
            crate::schema::RoleType::Tool => "tool",
        };
        
        let content = match &msg.content {
            crate::schema::Content::Text(s) => s.clone(),
            _ => String::new(),
        };
        
        let mut result = serde_json::json!({
            "role": role,
            "content": content,
        });
        
        if let Some(ref tool_call_id) = msg.tool_call_id {
            result["tool_call_id"] = serde_json::json!(tool_call_id);
        }
        
        if let Some(ref tool_calls) = msg.tool_calls {
            result["tool_calls"] = serde_json::json!(tool_calls.iter().map(|tc| serde_json::json!({
                "id": tc.id,
                "type": "function",
                "function": {
                    "name": tc.function.name,
                    "arguments": tc.function.arguments,
                }
            })).collect::<Vec<_>>());
        }
        
        result
    }
    
    fn response_to_message(&self, response: &OpenAIResponse) -> Result<crate::schema::Message, ModelError> {
        let choice = response.choices.first()
            .ok_or_else(|| ModelError::Api("No choices in response".to_string()))?;
        
        let content = choice.message.content.clone()
            .unwrap_or_default();
        
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls.iter().map(|call| {
                crate::schema::ToolCall {
                    id: call.id.clone(),
                    tool_type: call.tool_type.clone(),
                    function: crate::schema::FunctionCall {
                        name: call.function.name.clone(),
                        arguments: call.function.arguments.clone(),
                    },
                }
            }).collect()
        });
        
        Ok(crate::schema::Message {
            role: crate::schema::RoleType::Assistant,
            content: crate::schema::Content::Text(content),
            name: None,
            tool_call_id: None,
            tool_calls,
        })
    }
}

#[async_trait]
impl BaseChatModel for OpenAIChatModel {
    async fn generate(
        &self,
        input: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> Result<crate::schema::Message, ModelError> {
        let request = self.build_request(input, options).await;
        
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ModelError::Api(format!("HTTP {}: {}", status, body)));
        }
        
        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;
        
        self.response_to_message(&openai_response)
    }
    
    async fn stream(
        &self,
        input: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> Result<StreamReader, ModelError> {
        let request = self.build_request(input, options).await;
        
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ModelError::Api(format!("HTTP {}: {}", status, body)));
        }
        
        let (writer, reader) = StreamReader::new(100);
        
        let text = response.text().await.map_err(|e| ModelError::Api(e.to_string()))?;
        
        tokio::spawn(async move {
            let mut current_content = String::new();
            
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        continue;
                    }
                    
                    if let Ok(delta) = serde_json::from_str::<OpenAIDeltaResponse>(data) {
                        if let Some(choice) = delta.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                current_content.push_str(content);
                                
                                let msg = crate::schema::Message {
                                    role: crate::schema::RoleType::Assistant,
                                    content: crate::schema::Content::Text(current_content.clone()),
                                    name: None,
                                    tool_call_id: None,
                                    tool_calls: None,
                                };
                                let _ = writer.send_value(msg).await;
                            }
                        }
                    }
                }
            }
        });
        
        Ok(reader)
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    tools: Option<Vec<serde_json::Value>>,
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: usize,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIMessage {
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIDeltaResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIDeltaChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDeltaChoice {
    index: usize,
    delta: OpenAIMessage,
    finish_reason: Option<String>,
}

pub struct OpenAIChatModelBuilder {
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
}

impl OpenAIChatModelBuilder {
    pub fn new() -> Self {
        Self {
            api_key: None,
            model: None,
            base_url: None,
        }
    }
    
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
    
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
    
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }
    
    pub fn build(self) -> Result<OpenAIChatModel, String> {
        let api_key = self.api_key.ok_or("api_key is required")?;
        let model = self.model.unwrap_or_else(|| "gpt-4".to_string());
        
        Ok(OpenAIChatModel::new(OpenAIConfig {
            api_key,
            model,
            base_url: self.base_url,
        }))
    }
}

impl Default for OpenAIChatModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::Arc;
use crate::transport::HttpClient;

pub struct OpenAIModel {
    http: Arc<dyn HttpClient>,
    api_key: String,
    base_url: String,
}

impl OpenAIModel {
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
impl BaseChatModel for OpenAIModel {
    async fn generate(&self, messages: Vec<Message>, options: GenerateOptions) -> Result<Message, ModelError> {
        use serde_json::json;

        let body = json!({
            "model": options.model.clone().unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            "messages": messages.iter().map(|m| json!({
                "role": m.role.as_str(),
                "content": match &m.content {
                    crate::schema::Content::Text(s) => s.clone(),
                    _ => String::new(),
                }
            })).collect::<Vec<_>>(),
        });

        let url = format!("{}/chat/completions", self.base_url);
        let resp: serde_json::Value = self.http.post_json_with_headers(
            &url,
            &[("Authorization", &format!("Bearer {}", self.api_key))],
            &body,
        ).await.map_err(|e| ModelError::Api(e.to_string()))?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ModelError::Api("Invalid OpenAI response".to_string()))?;

        Ok(Message::assistant(content))
    }

    async fn stream(&self, _messages: Vec<Message>, _options: GenerateOptions) -> Result<StreamReader, ModelError> {
        Err(ModelError::Api("OpenAIModel::stream not yet implemented".to_string()))
    }
}
