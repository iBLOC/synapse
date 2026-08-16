use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnePanelError {
    #[error("http error: {0}")]
    Http(String),
    #[error("api error: code={code}, message={message}")]
    Api { code: i64, message: String },
    #[error("config error: {0}")]
    Config(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone)]
pub struct OnePanelConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_ms: u64,
    pub skip_tls_verify: bool,
}

impl OnePanelConfig {
    pub fn from_env() -> Result<Self, OnePanelError> {
        let base_url = std::env::var("ONEPANEL_BASE_URL")
            .map_err(|_| OnePanelError::Config("ONEPANEL_BASE_URL not set".to_string()))?;
        let api_key = std::env::var("ONEPANEL_API_KEY")
            .map_err(|_| OnePanelError::Config("ONEPANEL_API_KEY not set".to_string()))?;
        let timeout_ms = std::env::var("ONEPANEL_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30000);
        let skip_tls_verify = std::env::var("ONEPANEL_SKIP_TLS_VERIFY")
            .ok()
            .map(|s| s == "true")
            .unwrap_or(false);

        Ok(Self {
            base_url,
            api_key,
            timeout_ms,
            skip_tls_verify,
        })
    }
}

#[derive(Clone)]
pub struct OnePanelClient {
    client: Client,
    config: OnePanelConfig,
}

impl OnePanelClient {
    pub fn new(config: OnePanelConfig) -> Result<Self, OnePanelError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .danger_accept_invalid_certs(config.skip_tls_verify)
            .build()
            .map_err(|e| OnePanelError::Http(format!("client build: {e}")))?;

        Ok(Self { client, config })
    }

    pub fn from_env() -> Result<Self, OnePanelError> {
        let config = OnePanelConfig::from_env()?;
        Self::new(config)
    }

    fn sign(&self, _method: &str, _path: &str) -> (String, String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let token_input = format!("1panel{}{}", self.config.api_key, timestamp);
        let digest = md5::compute(token_input.as_bytes());
        let token = format!("{:x}", digest);

        (timestamp, token)
    }

    pub async fn get(&self, path: &str) -> Result<serde_json::Value, OnePanelError> {
        self.request(reqwest::Method::GET, path, None::<()>).await
    }

    pub async fn post<B: Serialize>(&self, path: &str, body: B) -> Result<serde_json::Value, OnePanelError> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    async fn request<B: Serialize>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<B>,
    ) -> Result<serde_json::Value, OnePanelError> {
        let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);
        let (timestamp, token) = self.sign(method.as_str(), path);

        let mut req = self.client
            .request(method, &url)
            .header("1Panel-Timestamp", &timestamp)
            .header("1Panel-Token", &token)
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await
            .map_err(|e| OnePanelError::Http(format!("request: {e}")))?;

        let status = resp.status();
        let text = resp.text().await
            .map_err(|e| OnePanelError::Http(format!("read body: {e}")))?;

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| OnePanelError::Serialization(format!("parse json: {e}")))?;

        if !status.is_success() {
            let code = value.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            let message = value.get("message").and_then(|v| v.as_str()).unwrap_or(&text).to_string();
            return Err(OnePanelError::Api { code, message });
        }

        Ok(value)
    }
}
