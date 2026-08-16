//! HTTP transport for MCP (Model Context Protocol).
//!
//! Sends JSON-RPC messages via HTTP POST. Reuses the `HttpClient` trait
//! so the same code works with native (reqwest) or WASM (wasi::http) HTTP.

use std::sync::Mutex;

use serde_json::{json, Value};

use crate::transport::{HttpClient, HttpError, HttpRequest, HttpResponse};

use super::types::{
    JsonRpcRequest, JsonRpcResponse, McpCallToolParams, McpCallToolResult, McpClientCapabilities,
    McpClientInfo, McpInitializeParams, McpInitializeResult, McpListToolsResult, McpTool,
};
use super::{McpError, McpResult};

pub struct HttpMcpClient {
    base_url: String,
    http: Arc<dyn HttpClient>,
    next_id: Mutex<i64>,
    session_id: Mutex<Option<String>>,
}

impl HttpMcpClient {
    pub fn new(base_url: impl Into<String>, http: Arc<dyn HttpClient>) -> Self {
        Self {
            base_url: base_url.into(),
            http,
            next_id: Mutex::new(1),
            session_id: Mutex::new(None),
        }
    }

    fn next_id(&self) -> i64 {
        let mut id = self.next_id.lock().unwrap();
        let v = *id;
        *id += 1;
        v
    }

    async fn send(&self, method: &str, params: Value) -> McpResult<Value> {
        let id = self.next_id();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: json!(id),
            method: method.to_string(),
            params: Some(params),
        };
        let body = serde_json::to_string(&req)
            .map_err(|e| McpError::InvalidResponse(format!("serialize: {}", e)))?;

        let url = format!("{}/mcp", self.base_url.trim_end_matches('/'));
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json, text/event-stream".to_string()),
        ];
        if let Some(sid) = self.session_id.lock().unwrap().clone() {
            headers.push(("Mcp-Session-Id".to_string(), sid));
        }

        let http_req = HttpRequest {
            method: crate::transport::HttpMethod::Post,
            url,
            headers,
            body: Some(body.into_bytes()),
        };

        let response: HttpResponse = self.http.execute(http_req).await.map_err(|e| {
            McpError::Io(format!("HTTP request failed: {}", e))
        })?;

        if let Some(sid) = response.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("mcp-session-id")) {
            *self.session_id.lock().unwrap() = Some(sid.1.clone());
        }

        if !(200..300).contains(&response.status) {
            return Err(McpError::Io(format!(
                "HTTP {} {}",
                response.status,
                String::from_utf8_lossy(&response.body)
            )));
        }

        let body_text = String::from_utf8(response.body)
            .map_err(|e| McpError::InvalidResponse(format!("non-utf8 body: {}", e)))?;

        let parsed: JsonRpcResponse = parse_response_body(&body_text)?;

        if let Some(err) = parsed.error {
            return Err(McpError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        parsed.result.ok_or_else(|| McpError::InvalidResponse("no result".into()))
    }

    pub async fn initialize(&mut self) -> McpResult<McpInitializeResult> {
        let params = McpInitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: McpClientCapabilities::default(),
            client_info: McpClientInfo {
                name: "synapse".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };
        let result = self
            .send("initialize", serde_json::to_value(params).unwrap())
            .await?;
        let init: McpInitializeResult = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("parse initialize: {}", e)))?;
        Ok(init)
    }

    pub async fn list_tools(&self) -> McpResult<Vec<McpTool>> {
        let result = self.send("tools/list", json!({})).await?;
        let list: McpListToolsResult = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("parse tools: {}", e)))?;
        Ok(list.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> McpResult<McpCallToolResult> {
        let params = McpCallToolParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };
        let result = self
            .send("tools/call", serde_json::to_value(params).unwrap())
            .await?;
        serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("parse call result: {}", e)))
    }
}

fn parse_response_body(body: &str) -> McpResult<JsonRpcResponse> {
    let trimmed = body.trim_start();
    if let Some(rest) = trimmed.strip_prefix("data:") {
        let json_str = rest.trim();
        return serde_json::from_str(json_str)
            .map_err(|e| McpError::InvalidResponse(format!("SSE parse: {}", e)));
    }
    serde_json::from_str(body)
        .map_err(|e| McpError::InvalidResponse(format!("JSON parse: {}", e)))
}

use std::sync::Arc;

impl From<HttpError> for McpError {
    fn from(e: HttpError) -> Self {
        McpError::Io(format!("HTTP: {}", e))
    }
}
