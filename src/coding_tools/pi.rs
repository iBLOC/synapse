use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::coding_tools::traits::{CodingTool, CodingToolError};
use crate::coding_tools::types::*;

pub struct PiClient {
    config: CodingToolConfig,
    http: Client,
}

impl PiClient {
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
            .unwrap_or("http://localhost:8080")
    }

    fn api_key(&self) -> Result<&str, CodingToolError> {
        self.config
            .api_key
            .as_deref()
            .ok_or_else(|| CodingToolError::Auth("API key not configured".into()))
    }
}

#[async_trait]
impl CodingTool for PiClient {
    fn name(&self) -> &str {
        "pi"
    }

    fn description(&self) -> &str {
        "Pi AI programming assistant"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/complete", self.base_url());

        let body = json!({
            "file_path": request.file_path,
            "prefix": request.prefix,
            "suffix": request.suffix,
            "language": request.language,
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

        Ok(CompletionResponse {
            completion: result["completion"].as_str().unwrap_or("").to_string(),
            has_more: result["has_more"].as_bool().unwrap_or(false),
            alternatives: result["alternatives"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/chat", self.base_url());

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
            "messages": messages,
            "system_prompt": request.system_prompt,
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

        Ok(ChatResponse {
            content: result["content"].as_str().unwrap_or("").to_string(),
            has_code_changes: result["has_code_changes"].as_bool().unwrap_or(false),
            code_changes: vec![],
        })
    }

    async fn read_file(&self, request: ReadFileRequest) -> Result<String, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/files/read", self.base_url());

        let body = json!({
            "file_path": request.file_path,
            "start_line": request.start_line,
            "end_line": request.end_line,
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

        Ok(result["content"].as_str().unwrap_or("").to_string())
    }

    async fn write_file(&self, request: WriteFileRequest) -> Result<(), CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/files/write", self.base_url());

        let body = json!({
            "file_path": request.file_path,
            "content": request.content,
            "create_dirs": request.create_dirs,
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

        Ok(())
    }

    async fn search(&self, request: SearchRequest) -> Result<Vec<SearchResult>, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/search", self.base_url());

        let body = json!({
            "query": request.query,
            "file_pattern": request.file_pattern,
            "directory": request.directory,
            "max_results": request.max_results,
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

        let results = result["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(SearchResult {
                            file_path: v["file_path"].as_str()?.to_string(),
                            line_number: v["line_number"].as_u64()? as usize,
                            line_content: v["line_content"].as_str()?.to_string(),
                            match_score: v["match_score"].as_f64().unwrap_or(0.0) as f32,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn execute(&self, request: ExecRequest) -> Result<ExecResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/execute", self.base_url());

        let body = json!({
            "command": request.command,
            "working_dir": request.working_dir,
            "timeout_secs": request.timeout_secs,
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

        Ok(ExecResponse {
            exit_code: result["exit_code"].as_i64().unwrap_or(-1) as i32,
            stdout: result["stdout"].as_str().unwrap_or("").to_string(),
            stderr: result["stderr"].as_str().unwrap_or("").to_string(),
        })
    }

    async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse, CodingToolError> {
        let api_key = self.api_key()?;
        let url = format!("{}/api/analyze", self.base_url());

        let body = json!({
            "file_path": request.file_path,
            "content": request.content,
            "analysis_type": match request.analysis_type {
                AnalysisType::Review => "review",
                AnalysisType::Refactor => "refactor",
                AnalysisType::Explain => "explain",
                AnalysisType::Test => "test",
                AnalysisType::Document => "document",
            },
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

        Ok(AnalyzeResponse {
            summary: result["summary"].as_str().unwrap_or("").to_string(),
            issues: vec![],
            suggestions: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_client_name() {
        let config = CodingToolConfig::default();
        let client = PiClient::new(config);
        assert_eq!(client.name(), "pi");
    }
}
