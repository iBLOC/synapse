use crate::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use crate::components::model::StreamReader;
use crate::schema::Message;
use crate::transport::HttpClient;
use crate::transport::HttpClientSse;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct OpenAIClient {
    http: Arc<dyn HttpClientSse>,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
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

    fn build_request_body(
        &self,
        messages: &[Message],
        options: &GenerateOptions,
    ) -> serde_json::Value {
        let mut body = json!({
            "model": options.model.clone().unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            "messages": messages.iter().map(|m| json!({
                "role": m.role.as_str(),
                "content": match &m.content {
                    crate::schema::Content::Text(s) => s.clone(),
                    _ => String::new(),
                }
            })).collect::<Vec<_>>(),
        });

        if let Some(temperature) = options.temperature {
            body["temperature"] = json!(temperature);
        }
        if let Some(max_tokens) = options.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        body
    }
}

#[async_trait]
impl BaseChatModel for OpenAIClient {
    async fn generate(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> ModelResult<Message> {
        let body = self.build_request_body(&messages, &options);
        let url = format!("{}/chat/completions", self.base_url);

        let http_client: &dyn HttpClient = self.http.as_ref();
        let resp: serde_json::Value = http_client
            .post_json_with_headers(
                &url,
                &[("Authorization", &format!("Bearer {}", self.api_key))],
                &body,
            )
            .await
            .map_err(|e| ModelError::Api(e.to_string()))?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ModelError::Api("Invalid OpenAI response format".to_string()))?;

        Ok(Message::assistant(content))
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        options: GenerateOptions,
    ) -> ModelResult<StreamReader> {
        let mut body = self.build_request_body(&messages, &options);
        body["stream"] = json!(true);

        let url = format!("{}/chat/completions", self.base_url);
        let api_key = self.api_key.clone();

        let sse = self
            .http
            .sse_post(
                &url,
                vec![
                    ("Authorization".to_string(), format!("Bearer {}", api_key)),
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

                if event.data == "[DONE]" {
                    let _ = writer
                        .send_value(Message::assistant(std::mem::take(&mut accumulated)))
                        .await;
                    return;
                }

                let v: serde_json::Value = match serde_json::from_str(&event.data) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = writer
                            .send_error(ModelError::StreamError(format!("JSON parse error: {e}")))
                            .await;
                        return;
                    }
                };

                if let Some(delta) = v
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    accumulated.push_str(delta);
                }
            }
        });

        Ok(reader)
    }
}
