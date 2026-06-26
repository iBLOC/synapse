//! HTTP transport abstraction for cross-platform agent communication.
//!
//! ## Overview
//!
//! PFC (前额叶) loads agents in WASM sandboxes. WASM agents need to:
//! - Call LLMs (Neuron / OpenAI / Anthropic)
//! - Make A2A protocol calls
//!
//! Both require HTTP. This module provides a unified `HttpClient` trait that
//! abstracts over native (reqwest) and WASM (wasi::http) implementations.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("request error: {0}")]
    Request(String),
    #[error("response error: {0}")]
    Response(String),
    #[error("status error: {0} {1}")]
    Status(u16, String),
    #[error("decode error: {0}")]
    Decode(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError>;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = native::ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub mod wasi;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = wasi::WasiHttpClient;

impl dyn HttpClient {
    pub async fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<R, HttpError> {
        let json = serde_json::to_vec(body).map_err(|e| HttpError::Request(e.to_string()))?;
        let req = HttpRequest {
            method: HttpMethod::Post,
            url: url.to_string(),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: Some(json),
        };
        let resp = self.execute(req).await?;
        if !(200..300).contains(&resp.status) {
            let body_str = String::from_utf8_lossy(&resp.body).into_owned();
            return Err(HttpError::Status(resp.status, body_str));
        }
        serde_json::from_slice(&resp.body).map_err(|e| HttpError::Decode(e.to_string()))
    }

    pub async fn post_json_with_headers<T, R>(
        &self,
        url: &str,
        extra_headers: &[(&str, &str)],
        body: &T,
    ) -> Result<R, HttpError>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let json = serde_json::to_vec(body).map_err(|e| HttpError::Request(e.to_string()))?;
        let mut headers: Vec<(String, String)> =
            vec![("Content-Type".into(), "application/json".into())];
        for (k, v) in extra_headers {
            headers.push((k.to_string(), v.to_string()));
        }
        let req = HttpRequest {
            method: HttpMethod::Post,
            url: url.to_string(),
            headers,
            body: Some(json),
        };
        let resp = self.execute(req).await?;
        if !(200..300).contains(&resp.status) {
            let body_str = String::from_utf8_lossy(&resp.body).into_owned();
            return Err(HttpError::Status(resp.status, body_str));
        }
        serde_json::from_slice(&resp.body).map_err(|e| HttpError::Decode(e.to_string()))
    }
}
