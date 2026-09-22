//! HTTP client that goes through Ziti instead of the public internet.
//!
//! Use when you need to talk to a peer encephalon / tiangong service
//! whose service name only resolves inside the Ziti overlay. The
//! API is intentionally minimal — for richer HTTP needs, prefer
//! plugging a ZitiStream into your own hyper client.

use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::time::Duration;

use super::gateway::ZitiGateway;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub struct ZitiHttpClient {
    gateway: Arc<ZitiGateway>,
    timeout: Duration,
}

impl ZitiHttpClient {
    pub fn new(gateway: Arc<ZitiGateway>) -> Self {
        Self {
            gateway,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(gateway: Arc<ZitiGateway>, timeout: Duration) -> Self {
        Self { gateway, timeout }
    }

    pub async fn get(&self, service_name: &str, path: &str) -> Result<HttpResponse> {
        self.request(service_name, "GET", path, None, None).await
    }

    pub async fn post(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "POST", path, Some(body), None).await
    }

    pub async fn put(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "PUT", path, Some(body), None).await
    }

    pub async fn delete(&self, service_name: &str, path: &str) -> Result<HttpResponse> {
        self.request(service_name, "DELETE", path, None, None).await
    }

    pub async fn patch(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "PATCH", path, Some(body), None).await
    }

    pub async fn request(
        &self,
        service_name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<HttpResponse> {
        let stream = tokio::time::timeout(self.timeout, self.gateway.dial(service_name)).await??;

        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::error!("Connection error: {}", e);
            }
        });

        let mut req_builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Host", service_name);

        if let Some(headers) = headers {
            for (key, value) in headers {
                req_builder = req_builder.header(&key, &value);
            }
        }

        let req = if let Some(body) = body {
            req_builder
                .header("Content-Type", "application/json")
                .header("Content-Length", body.len())
                .body(Full::new(Bytes::from(body.to_string())))?
        } else {
            req_builder.body(Full::new(Bytes::new()))?
        };

        let resp = tokio::time::timeout(self.timeout, sender.send_request(req)).await??;

        let status = resp.status();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.collect().await?.to_bytes();

        Ok(HttpResponse {
            status,
            headers,
            body: String::from_utf8(body.to_vec())?,
        })
    }
}