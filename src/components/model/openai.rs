use crate::components::model::{BaseChatModel, GenerateOptions, MessageDelta, ModelError, StreamReader};
use crate::schema::Message;
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
pub struct OpenAIChatModel {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl OpenAIChatModel {
    pub fn new(config: OpenAIConfig) -> Self {
        let base_url = config
            .base_url
            .unwrap_or_else(|| "https://api.openai.com".to_string());

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

    async fn build_request(
        &self,
        messages: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> serde_json::Value {
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
                request["tools"] = serde_json::json!(tools
                    .iter()
                    .map(|t| serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    }))
                    .collect::<Vec<_>>());
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
            result["tool_calls"] = serde_json::json!(tool_calls
                .iter()
                .map(|tc| serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.function.name,
                        "arguments": tc.function.arguments,
                    }
                }))
                .collect::<Vec<_>>());
        }

        result
    }

    fn response_to_message(
        &self,
        response: &OpenAIResponse,
    ) -> Result<crate::schema::Message, ModelError> {
        let choice = response
            .choices
            .first()
            .ok_or_else(|| ModelError::Api("No choices in response".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();

        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|call| crate::schema::ToolCall {
                    id: call.id.clone(),
                    tool_type: call.tool_type.clone(),
                    function: crate::schema::FunctionCall {
                        name: call.function.name.clone(),
                        arguments: call.function.arguments.clone(),
                    },
                })
                .collect()
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
#[cfg(not(target_arch = "wasm32"))]
impl BaseChatModel for OpenAIChatModel {
    async fn generate(
        &self,
        input: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> Result<crate::schema::Message, ModelError> {
        let request = self.build_request(input, options).await;

        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = self
            .client
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

        let response = self
            .client
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

        let text = response
            .text()
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;

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

#[cfg(not(target_arch = "wasm32"))]
pub struct OpenAIChatModelBuilder {
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
impl Default for OpenAIChatModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

use crate::transport::{HttpClient, HttpClientSse};
use std::sync::Arc;

pub struct OpenAIModel {
    http: Arc<dyn HttpClientSse>,
    api_key: String,
    base_url: String,
}

impl OpenAIModel {
    pub fn new(http: Arc<dyn HttpClientSse>, api_key: impl Into<String>) -> Self {
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
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> Result<Message, ModelError> {
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
        let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
        let req = crate::transport::HttpRequest {
            method: crate::transport::HttpMethod::Post,
            url: url.clone(),
            headers: vec![
                ("Authorization".to_string(), format!("Bearer {}", self.api_key)),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(body_bytes),
        };
        let http_resp = self
            .http
            .execute(req)
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;
        let resp: serde_json::Value = serde_json::from_slice(&http_resp.body)
            .map_err(|e| ModelError::Api(e.to_string()))?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ModelError::Api("Invalid OpenAI response".to_string()))?;

        Ok(Message::assistant(content))
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> Result<StreamReader, ModelError> {
        Err(ModelError::Api(
            "OpenAIModel::stream not yet implemented; use stream_deltas".to_string(),
        ))
    }

    async fn stream_deltas(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> Result<crate::stream::StreamReader<MessageDelta>, ModelError> {
        use crate::components::model::delta::{FinishReason, MessageDelta, ToolCallDelta};
        use crate::transport::sse::parse_chunk;
        use serde_json::Value;
        use tokio::sync::mpsc;

        let model_name = options
            .model
            .clone()
            .unwrap_or_else(|| "gpt-3.5-turbo".to_string());

        let body = serde_json::json!({
            "model": model_name,
            "stream": true,
            "stream_options": { "include_usage": true },
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": m.role.as_str(),
                "content": match &m.content {
                    crate::schema::Content::Text(s) => s.clone(),
                    _ => String::new(),
                }
            })).collect::<Vec<_>>(),
        });

        let url = format!("{}/chat/completions", self.base_url);
        let http = self.http.clone();
        let api_key = self.api_key.clone();

        let (writer, reader) = crate::stream::StreamReader::<MessageDelta>::new(64);
        let cancel = reader.cancel_handle();

        // Run on a separate task; the sse_post stream is consumed in a loop.
        tokio::spawn(async move {
            let sse_result = http
                .sse_post(
                    &url,
                    vec![
                        ("Authorization".to_string(), format!("Bearer {}", api_key)),
                        ("Content-Type".to_string(), "application/json".to_string()),
                    ],
                    serde_json::to_vec(&body).unwrap_or_default(),
                )
                .await;

            let mut sse = match sse_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = writer
                        .send(MessageDelta {
                            content_delta: String::new(),
                            tool_call_deltas: Vec::new(),
                            finish_reason: Some(FinishReason::Error),
                            usage: None,
                        })
                        .await;
                    let _ = writer
                        .send(MessageDelta {
                            content_delta: format!("sse_post failed: {e}"),
                            tool_call_deltas: Vec::new(),
                            finish_reason: Some(FinishReason::Error),
                            usage: None,
                        })
                        .await;
                    return;
                }
            };

            let mut state = crate::transport::sse::SseParseState::new();
            while let Some(event_res) = sse.next().await {
                if cancel.is_cancelled() {
                    return;
                }
                let event = match event_res {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = writer
                            .send(MessageDelta {
                                content_delta: format!("sse error: {e}"),
                                tool_call_deltas: Vec::new(),
                                finish_reason: Some(FinishReason::Error),
                                usage: None,
                            })
                            .await;
                        return;
                    }
                };

                for ev in parse_chunk(event.data.as_bytes(), &mut state) {
                    if ev.data.trim().is_empty() || ev.data.trim() == "[DONE]" {
                        if ev.data.trim() == "[DONE]" {
                            let _ = writer
                                .send(MessageDelta {
                                    content_delta: String::new(),
                                    tool_call_deltas: Vec::new(),
                                    finish_reason: Some(FinishReason::Stop),
                                    usage: None,
                                })
                                .await;
                        }
                        continue;
                    }
                    let parsed: Value = match serde_json::from_str(&ev.data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let delta = parse_openai_stream_chunk(&parsed);
                    if !writer.send(delta).await {
                        return;
                    }
                }
            }
        });

        Ok(reader)
    }
}

fn parse_openai_stream_chunk(v: &serde_json::Value) -> crate::components::model::delta::MessageDelta {
    use crate::components::model::delta::{FinishReason, MessageDelta, ToolCallDelta, Usage};

    let mut content_delta = String::new();
    let mut tool_call_deltas: Vec<ToolCallDelta> = Vec::new();
    let mut finish_reason: Option<FinishReason> = None;
    let mut usage: Option<Usage> = None;

    if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
        for choice in choices {
            if let Some(delta) = choice.get("delta") {
                if let Some(c) = delta.get("content").and_then(|c| c.as_str()) {
                    content_delta.push_str(c);
                }
                if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tcs {
                        let index = tc
                            .get("index")
                            .and_then(|i| i.as_u64())
                            .unwrap_or(0) as usize;
                        let id = tc
                            .get("id")
                            .and_then(|i| i.as_str())
                            .map(String::from);
                        let name = tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            .map(String::from);
                        let arguments_delta = tc
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|a| a.as_str())
                            .unwrap_or("")
                            .to_string();
                        tool_call_deltas.push(ToolCallDelta {
                            index,
                            id,
                            name,
                            arguments_delta,
                        });
                    }
                }
            }
            if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                finish_reason = Some(match fr {
                    "stop" => FinishReason::Stop,
                    "tool_calls" => FinishReason::ToolCalls,
                    "length" => FinishReason::Length,
                    "content_filter" => FinishReason::ContentFilter,
                    _ => FinishReason::Error,
                });
            }
        }
    }

    if let Some(u) = v.get("usage") {
        usage = Some(Usage {
            input_tokens: u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            output_tokens: u.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0)
                as u32,
            cached_input_tokens: u
                .get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as u32,
        });
    }

    MessageDelta {
        content_delta,
        tool_call_deltas,
        finish_reason,
        usage,
    }
}

#[cfg(test)]
mod delta_parser_tests {
    use super::*;
    use crate::components::model::delta::FinishReason;

    #[test]
    fn parses_text_delta() {
        let v = serde_json::json!({
            "choices": [{
                "delta": {"content": "Hello"}
            }]
        });
        let d = parse_openai_stream_chunk(&v);
        assert_eq!(d.content_delta, "Hello");
        assert!(d.tool_call_deltas.is_empty());
        assert!(d.finish_reason.is_none());
    }

    #[test]
    fn parses_tool_call_delta() {
        let v = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_123",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"loc"
                        }
                    }]
                }
            }]
        });
        let d = parse_openai_stream_chunk(&v);
        assert_eq!(d.content_delta, "");
        assert_eq!(d.tool_call_deltas.len(), 1);
        assert_eq!(d.tool_call_deltas[0].index, 0);
        assert_eq!(d.tool_call_deltas[0].id.as_deref(), Some("call_123"));
        assert_eq!(d.tool_call_deltas[0].name.as_deref(), Some("get_weather"));
        assert_eq!(d.tool_call_deltas[0].arguments_delta, "{\"loc");
    }

    #[test]
    fn parses_finish_reason() {
        let v = serde_json::json!({
            "choices": [{
                "delta": {},
                "finish_reason": "tool_calls"
            }]
        });
        let d = parse_openai_stream_chunk(&v);
        assert_eq!(d.finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn parses_usage_when_present() {
        let v = serde_json::json!({
            "choices": [{"delta": {}}],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 34,
                "prompt_tokens_details": {"cached_tokens": 5}
            }
        });
        let d = parse_openai_stream_chunk(&v);
        let u = d.usage.expect("usage should be set");
        assert_eq!(u.input_tokens, 12);
        assert_eq!(u.output_tokens, 34);
        assert_eq!(u.cached_input_tokens, 5);
    }

    #[test]
    fn missing_fields_yield_empty_delta() {
        let v = serde_json::json!({});
        let d = parse_openai_stream_chunk(&v);
        assert!(d.content_delta.is_empty());
        assert!(d.tool_call_deltas.is_empty());
        assert!(d.finish_reason.is_none());
        assert!(d.usage.is_none());
    }
}
