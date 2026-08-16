use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::coding_tools::traits::{CodingTool, CodingToolError};
use crate::coding_tools::types::*;

pub struct ClaudeCodeClient {
    config: CodingToolConfig,
    http: Client,
}

impl ClaudeCodeClient {
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
            .unwrap_or("https://api.anthropic.com")
    }

    fn api_key(&self) -> Result<&str, CodingToolError> {
        self.config
            .api_key
            .as_deref()
            .ok_or_else(|| CodingToolError::Auth("Anthropic API key not configured".into()))
    }

    fn model(&self) -> &str {
        self.config
            .model
            .as_deref()
            .unwrap_or("claude-3-5-sonnet-20241022")
    }
}

#[async_trait]
impl CodingTool for ClaudeCodeClient {
    fn name(&self) -> &str {
        "claude_code"
    }

    fn description(&self) -> &str {
        "Anthropic Claude Code programming assistant"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, CodingToolError> {
        let prompt = format!(
            "Complete the following {} code:\n\n```{}\n{}\n```{}",
            request.language.as_deref().unwrap_or("unknown"),
            request.language.as_deref().unwrap_or(""),
            request.prefix,
            request
                .suffix
                .map(|s| format!("\n\nAfter cursor:\n```{}\n{}\n```", request.language.as_deref().unwrap_or(""), s))
                .unwrap_or_default()
        );

        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            file_context: None,
            system_prompt: Some(
                "You are an expert programmer. Complete the code concisely.".into(),
            ),
        };

        let response = self.chat(chat_request).await?;

        Ok(CompletionResponse {
            completion: response.content,
            has_more: false,
            alternatives: vec![],
        })
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/v1/messages", self.base_url());

        let mut messages: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            match msg.role {
                ChatRole::User => {
                    messages.push(json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                ChatRole::Assistant => {
                    messages.push(json!({
                        "role": "assistant",
                        "content": msg.content,
                    }));
                }
                ChatRole::System => {
                    messages.push(json!({
                        "role": "user",
                        "content": format!("<system>{}</system>", msg.content),
                    }));
                }
            }
        }

        let body = json!({
            "model": self.model(),
            "max_tokens": self.config.max_tokens.unwrap_or(4096),
            "messages": messages,
        });

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
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

        let content = result["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
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
            "Claude Code does not support direct file reading via API".into(),
        ))
    }

    async fn write_file(&self, _request: WriteFileRequest) -> Result<(), CodingToolError> {
        Err(CodingToolError::Api(
            "Claude Code does not support direct file writing via API".into(),
        ))
    }

    async fn search(&self, _request: SearchRequest) -> Result<Vec<SearchResult>, CodingToolError> {
        Err(CodingToolError::Api(
            "Claude Code does not support code search via API".into(),
        ))
    }

    async fn execute(&self, _request: ExecRequest) -> Result<ExecResponse, CodingToolError> {
        Err(CodingToolError::Api(
            "Claude Code does not support command execution via API".into(),
        ))
    }

    async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse, CodingToolError> {
        let content = request.content.unwrap_or_default();
        let prompt = match request.analysis_type {
            AnalysisType::Review => format!("Review this code and provide feedback:\n\n{}", content),
            AnalysisType::Refactor => {
                format!("Suggest refactoring improvements for:\n\n{}", content)
            }
            AnalysisType::Explain => format!("Explain what this code does:\n\n{}", content),
            AnalysisType::Test => {
                format!("Write comprehensive tests for:\n\n{}", content)
            }
            AnalysisType::Document => {
                format!("Write detailed documentation for:\n\n{}", content)
            }
        };

        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
            }],
            file_context: None,
            system_prompt: Some(
                "You are an expert code analyst. Provide detailed, actionable feedback.".into(),
            ),
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
    fn test_claude_code_client_name() {
        let config = CodingToolConfig::default();
        let client = ClaudeCodeClient::new(config);
        assert_eq!(client.name(), "claude_code");
    }
}
