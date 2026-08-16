use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{ChatOptions, Content, ContentPart, LLMError, LLMProvider, LLMResult, Message, Stream};

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

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message> {
        let model = options.model.unwrap_or_else(|| self.default_model.clone());

        let request = OpenAIChatRequest {
            model: model.clone(),
            messages: messages
                .iter()
                .map(|m| OpenAIMessage {
                    role: match m.role {
                        super::Role::System => "system".to_string(),
                        super::Role::User => "user".to_string(),
                        super::Role::Assistant => "assistant".to_string(),
                        super::Role::Tool => "tool".to_string(),
                    },
                    content: m.content.text.clone().unwrap_or_default(),
                })
                .collect(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
            stop: options.stop,
            tools: options.tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.input_schema,
                            }
                        })
                    })
                    .collect()
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

        let response = self
            .client
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

        let choice = response
            .choices
            .first()
            .ok_or_else(|| LLMError::Api("No choices in response".to_string()))?;

        let text = choice.message.content.clone().unwrap_or_default();
        let mut parts: Vec<ContentPart> = Vec::new();

        if !text.is_empty() {
            parts.push(ContentPart::Text { text });
        }

        if let Some(ref tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                let input: serde_json::Value = tc
                    .function
                    .arguments
                    .as_deref()
                    .and_then(|args| serde_json::from_str(args).ok())
                    .unwrap_or(serde_json::json!({}));
                parts.push(ContentPart::ToolUse {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    input,
                });
            }
        }

        Ok(Message {
            role: super::Role::Assistant,
            content: if parts.is_empty() {
                Content::text(String::new())
            } else {
                Content::from_parts(parts)
            },
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
        let request = serde_json::json!({
            "input": texts,
            "model": "text-embedding-3-small",
        });

        let response = self
            .client
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
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    _type: Option<String>,
    function: OpenAIToolCallFunction,
}

#[derive(Deserialize)]
struct OpenAIToolCallFunction {
    name: String,
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_tool_call_response() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Paris\"}"
                        }
                    }]
                }
            }]
        });
        let response: OpenAIChatResponse =
            serde_json::from_value(json).expect("must deserialize");
        let msg = &response.choices[0].message;

        assert_eq!(msg.content.as_deref(), None);
        let tc = msg.tool_calls.as_ref().unwrap();
        assert_eq!(tc.len(), 1);
        assert_eq!(tc[0].id, "call_abc");
        assert_eq!(tc[0].function.name, "get_weather");
        assert_eq!(
            tc[0].function.arguments.as_deref(),
            Some("{\"city\":\"Paris\"}")
        );
    }

    #[test]
    fn deserialize_text_only_response() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello!",
                    "tool_calls": null
                }
            }]
        });
        let response: OpenAIChatResponse =
            serde_json::from_value(json).expect("must deserialize");
        let msg = &response.choices[0].message;

        assert_eq!(msg.content.as_deref(), Some("Hello!"));
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn deserialize_multiple_tool_calls() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [
                        {"id": "c1", "type": "function", "function": {"name": "f1", "arguments": "{}"}},
                        {"id": "c2", "type": "function", "function": {"name": "f2", "arguments": "{\"k\":\"v\"}"}}
                    ]
                }
            }]
        });
        let response: OpenAIChatResponse =
            serde_json::from_value(json).expect("must deserialize");
        let tc = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tc.len(), 2);
        assert_eq!(tc[0].id, "c1");
        assert_eq!(tc[1].id, "c2");
    }

    #[test]
    fn tool_use_builds_content_parts_from_tool_call() {
        use super::ContentPart;
        let tc = OpenAIToolCall {
            id: "call_1".into(),
            _type: Some("function".into()),
            function: OpenAIToolCallFunction {
                name: "search".into(),
                arguments: Some("{\"term\":\"rust\"}".into()),
            },
        };
        let input: serde_json::Value = tc
            .function
            .arguments
            .as_deref()
            .and_then(|args| serde_json::from_str(args).ok())
            .unwrap_or(serde_json::json!({}));
        let part = ContentPart::ToolUse {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            input,
        };

        assert!(matches!(part, ContentPart::ToolUse { .. }));
        if let ContentPart::ToolUse { id, name, ref input } = part {
            assert_eq!(id, "call_1");
            assert_eq!(name, "search");
            assert_eq!(input["term"], "rust");
        } else {
            unreachable!();
        }
    }

    #[test]
    fn tool_use_parses_empty_arguments_as_empty_object() {
        let input: serde_json::Value =
            serde_json::from_str("").unwrap_or(serde_json::json!({}));
        assert_eq!(input, serde_json::json!({}));
    }
}
