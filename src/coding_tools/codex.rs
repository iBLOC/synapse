use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::coding_tools::traits::{CodingTool, CodingToolError};
use crate::coding_tools::types::*;

pub struct CodexClient {
    config: CodingToolConfig,
    http: Client,
}

impl CodexClient {
    pub fn new(config: CodingToolConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.timeout_secs.unwrap_or(30),
            ))
            .build()
            .expect("Failed to create HTTP client");
        Self { config, http }
    }

    fn base_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
    }

    fn api_key(&self) -> Result<&str, CodingToolError> {
        self.config
            .api_key
            .as_deref()
            .ok_or_else(|| CodingToolError::Auth("OpenAI API key not configured".into()))
    }

    fn model(&self) -> &str {
        self.config.model.as_deref().unwrap_or("codex-mini")
    }
}

#[async_trait]
impl CodingTool for CodexClient {
    fn name(&self) -> &str {
        "codex"
    }

    fn description(&self) -> &str {
        "OpenAI Codex CLI programming assistant"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/completions", self.base_url());

        let prompt = format!(
            "Complete the following code:\n\n{}\n{}",
            request.prefix,
            request.suffix.unwrap_or_default()
        );

        let body = json!({
            "model": self.model(),
            "prompt": prompt,
            "max_tokens": self.config.max_tokens.unwrap_or(1000),
            "temperature": self.config.temperature.unwrap_or(0.2),
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| CodingToolError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CodingToolError::Api(format!("{} {}", status, text)));
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| CodingToolError::Api(e.to_string()))?;

        let completion = result["choices"]
            .get(0)
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(CompletionResponse {
            completion,
            has_more: false,
            alternatives: vec![],
        })
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/chat/completions", self.base_url());

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                        ChatRole::System => "system",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = json!({
            "model": self.model(),
            "messages": messages,
            "max_tokens": self.config.max_tokens.unwrap_or(2000),
            "temperature": self.config.temperature.unwrap_or(0.3),
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| CodingToolError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CodingToolError::Api(format!("{} {}", status, text)));
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| CodingToolError::Api(e.to_string()))?;

        let content = result["choices"]
            .get(0)
            .and_then(|c| c["message"]["content"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            has_code_changes: false,
            code_changes: vec![],
        })
    }

    async fn read_file(&self, _request: ReadFileRequest) -> Result<String, CodingToolError> {
        Err(CodingToolError::Api(
            "Codex does not support direct file reading".into(),
        ))
    }

    async fn write_file(&self, _request: WriteFileRequest) -> Result<(), CodingToolError> {
        Err(CodingToolError::Api(
            "Codex does not support direct file writing".into(),
        ))
    }

    async fn search(&self, _request: SearchRequest) -> Result<Vec<SearchResult>, CodingToolError> {
        Err(CodingToolError::Api(
            "Codex does not support code search".into(),
        ))
    }

    async fn execute(&self, _request: ExecRequest) -> Result<ExecResponse, CodingToolError> {
        Err(CodingToolError::Api(
            "Codex does not support command execution".into(),
        ))
    }

    async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse, CodingToolError> {
        let content = request.content.unwrap_or_default();
        let prompt = match request.analysis_type {
            AnalysisType::Review => format!("Review this code:\n\n{}", content),
            AnalysisType::Refactor => format!("Suggest refactoring for:\n\n{}", content),
            AnalysisType::Explain => format!("Explain this code:\n\n{}", content),
            AnalysisType::Test => format!("Write tests for:\n\n{}", content),
            AnalysisType::Document => format!("Write documentation for:\n\n{}", content),
        };

        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            file_context: None,
            system_prompt: Some("You are an expert code analyst.".into()),
        };

        let response = self.chat(chat_request).await?;

        Ok(AnalyzeResponse {
            summary: response.content,
            issues: vec![],
            suggestions: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_client_name() {
        let config = CodingToolConfig::default();
        let client = CodexClient::new(config);
        assert_eq!(client.name(), "codex");
    }
}
