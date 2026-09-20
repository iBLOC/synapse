use super::{HttpClient, HttpError, HttpMethod, HttpRequest, HttpResponse};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

pub struct ReqwestHttpClient {
    client: Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Build an HTTP client with the given per-request timeout. Returns
    /// an error if the underlying reqwest builder rejects the
    /// configuration so the caller can fail loudly at startup instead
    /// of silently substituting an unconfigured client that ignores
    /// the timeout.
    pub fn try_with_timeout(timeout: Duration) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(timeout)
            .build()?;
        Ok(Self { client })
    }

    /// Infallible wrapper around `try_with_timeout` — used at boot to
    /// surface a startup error if the timeout can't be applied. Today
    /// reqwest's builder only fails on URI/header misconfiguration
    /// (timeout itself never fails), so the unwrap is safe; kept
    /// explicit so a future reqwest change is caught loudly.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self::try_with_timeout(timeout).expect("reqwest client with timeout")
    }

    /// 访问底层 reqwest 客户端（用于不走 LLMProvider 抽象的业务 HTTP 调用）。
    ///
    /// 业务客户端（如 `cortex/hippocampus` 内部 API）可以直接用 reqwest
    /// 的 fluent API，而 LLM/audio 调用应走 `OpenAIProvider` / `AsrProvider`。
    pub fn raw(&self) -> &Client {
        &self.client
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let mut builder = match req.method {
            HttpMethod::Get => self.client.get(&req.url),
            HttpMethod::Post => self.client.post(&req.url),
            HttpMethod::Put => self.client.put(&req.url),
            HttpMethod::Delete => self.client.delete(&req.url),
        };
        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }
        if let Some(body) = req.body {
            builder = builder.body(body);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .bytes()
            .await
            .map_err(|e| HttpError::Response(e.to_string()))?
            .to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
