use crate::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use crate::components::model::StreamReader;
use crate::schema::{Content, Message, RoleType};
use crate::transport::HttpClient;
use crate::transport::HttpClientSse;
use async_trait::async_trait;
use std::sync::Arc;
use serde_json::json;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct AnthropicClient {
    http: Arc<dyn HttpClientSse>,
    api_key: String,
    base_url: String,
    model: String,
    max_tokens: u32,
}

impl AnthropicClient {
    pub fn new(http: Arc<dyn HttpClientSse>, api_key: impl Into<String>) -> Self {
        Self {
            http,
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    fn build_request_body(&self, messages: &[Message]) -> (Option<String>, serde_json::Value) {
        let mut system_text: Option<String> = None;
        let mut api_messages: Vec<serde_json::Value> = Vec::with_capacity(messages.len());

        for m in messages {
            let content = match &m.content {
                Content::Text(s) => s.clone(),
                _ => String::new(),
            };
            match m.role {
                RoleType::System => {
                    system_text = Some(content);
                }
                _ => {
                    api_messages.push(json!({
                        "role": m.role.as_str(),
                        "content": content,
                    }));
                }
            }
        }

        let body = match &system_text {
            Some(sys) => json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "system": sys,
                "messages": api_messages,
            }),
            None => json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "messages": api_messages,
            }),
        };
        (system_text, body)
    }
}

#[async_trait]
impl BaseChatModel for AnthropicClient {
    async fn generate(
        &self,
        messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<Message> {
        let (_system, body) = self.build_request_body(&messages);
        let url = format!("{}/v1/messages", self.base_url);

        let http_client: &dyn HttpClient = self.http.as_ref();
        let resp: serde_json::Value = http_client
            .post_json_with_headers(
                &url,
                &[
                    ("x-api-key", &self.api_key),
                    ("anthropic-version", ANTHROPIC_VERSION),
                ],
                &body,
            )
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;

        if let Some(error_obj) = resp.get("error") {
            let msg = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(ModelError::Api(format!("Anthropic error: {}", msg)));
        }

        let content_blocks = resp["content"]
            .as_array()
            .ok_or_else(|| ModelError::Api("Invalid response: missing content".to_string()))?;

        let mut combined_text = String::new();
        for block in content_blocks {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    if !combined_text.is_empty() {
                        combined_text.push('\n');
                    }
                    combined_text.push_str(text);
                }
            }
        }

        if combined_text.is_empty() {
            return Err(ModelError::Api(
                "Anthropic returned empty content".to_string(),
            ));
        }

        Ok(Message::assistant(combined_text))
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<StreamReader> {
        let (_system, mut body) = self.build_request_body(&messages);
        body["stream"] = json!(true);

        let url = format!("{}/v1/messages", self.base_url);
        let api_key = self.api_key.clone();

        let sse = self
            .http
            .sse_post(
                &url,
                vec![
                    ("x-api-key".to_string(), api_key),
                    ("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
                    ("Content-Type".to_string(), "application/json".to_string()),
                ],
                serde_json::to_vec(&body).unwrap_or_default(),
            )
            .await
            .map_err(|e| ModelError::StreamError(e.to_string()))?;

        let (writer, reader) = StreamReader::new(64);

        tokio::spawn(async move {
            let mut sse = sse;
            let mut accumulated = String::new();
            while let Some(event_res) = sse.next().await {
                let event = match event_res {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = writer.send_error(ModelError::StreamError(e.to_string())).await;
                        return;
                    }
                };

                let v: serde_json::Value = match serde_json::from_str(&event.data) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = writer
                            .send_error(ModelError::StreamError(format!("JSON parse error: {e}")))
                            .await;
                        return;
                    }
                };

                let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

                match event_type {
                    "content_block_delta" => {
                        if let Some(text) = v
                            .get("delta")
                            .and_then(|d| d.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            accumulated.push_str(text);
                        }
                    }
                    "message_stop" => {
                        let _ = writer
                            .send_value(Message::assistant(std::mem::take(&mut accumulated)))
                            .await;
                        return;
                    }
                    "error" => {
                        let msg = v
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error");
                        let _ = writer
                            .send_error(ModelError::Api(format!("Anthropic error: {msg}")))
                            .await;
                        return;
                    }
                    _ => {}
                }
            }
        });

        Ok(reader)
    }
}
