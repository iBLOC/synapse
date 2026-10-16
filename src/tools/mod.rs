//! 通用工具集 — 跨平台、无设备依赖的"通用能力"层。
//!
//! 工具分级（2026-10-15 用户定调）：
//! - **大脑层**（本模块）：echo / time / json / http —— 任意宿主形态都有意义
//! - **设备层**（不在本模块）：shell / file / mobile_sensor —— 需用户授权或形态特定
//!
//! 设计要点：
//! - `Tool` trait + 5 个支撑类型（ToolError / ToolOutput / ToolMetadata /
//!   ToolRateLimitConfig / ToolRegistry）从 prefrontal 迁来，确保
//!   synapse 是工具的**宿主**而非反过来依赖。
//! - 注册接口与 prefrontal 兼容（`registry.register(tool)`），后续可平滑替换。
//! - 4 个通用工具用纯 std + synapse 自带 dep（chrono / chrono-tz / reqwest / serde）。
//!   time 工具支持 `timezone`（IANA 名，如 Asia/Shanghai），human 格式输出 `%Z` 缩写
//!   （2026-10-16：迁入时补齐，与 prefrontal 旧实现行为对齐）。

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod onepanel;

/// How much approval a specific tool invocation requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRequirement {
    Never,
    UnlessAutoApproved,
    Always,
}

impl ApprovalRequirement {
    pub fn is_required(&self) -> bool {
        !matches!(self, Self::Never)
    }
}

/// Approval context for autonomous tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalContext {
    Autonomous {
        allowed_tools: std::collections::HashSet<String>,
    },
}

impl ApprovalContext {
    pub fn autonomous() -> Self {
        Self::Autonomous {
            allowed_tools: std::collections::HashSet::new(),
        }
    }

    pub fn is_blocked(&self, tool_name: &str, requirement: ApprovalRequirement) -> bool {
        match self {
            Self::Autonomous { allowed_tools } => match requirement {
                ApprovalRequirement::Never => false,
                ApprovalRequirement::UnlessAutoApproved => false,
                ApprovalRequirement::Always => !allowed_tools.contains(tool_name),
            },
        }
    }
}

/// Per-tool rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
}

impl ToolRateLimitConfig {
    pub fn new(requests_per_minute: u32, requests_per_hour: u32) -> Self {
        Self {
            requests_per_minute,
            requests_per_hour,
        }
    }
}

impl Default for ToolRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
        }
    }
}

/// Where a tool should execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolDomain {
    Orchestrator,
    Container,
}

impl Default for ToolDomain {
    fn default() -> Self {
        Self::Orchestrator
    }
}

/// Error type for tool execution.
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum ToolError {
    #[error("invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("not authorized: {0}")]
    NotAuthorized(String),
    #[error("rate limited, retry after {0:?}")]
    RateLimited(Option<Duration>),
    #[error("external service error: {0}")]
    ExternalServiceError(String),
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("tool disabled: {0}")]
    ToolDisabled(String),
}

/// Metadata about a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub approval_requirement: ApprovalRequirement,
    pub rate_limit: Option<ToolRateLimitConfig>,
    pub domain: ToolDomain,
    pub read_only: bool,
}

impl ToolMetadata {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({}),
            approval_requirement: ApprovalRequirement::Never,
            rate_limit: None,
            domain: ToolDomain::default(),
            read_only: true,
        }
    }

    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn with_approval(mut self, approval: ApprovalRequirement) -> Self {
        self.approval_requirement = approval;
        self
    }

    pub fn with_rate_limit(mut self, limit: ToolRateLimitConfig) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn with_domain(mut self, domain: ToolDomain) -> Self {
        self.domain = domain;
        self
    }

    pub fn with_write(mut self) -> Self {
        self.read_only = false;
        self
    }
}

/// A tool that can be called by the agent.
#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError>;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(self.name(), self.description()).with_schema(self.input_schema())
    }
}

/// A tool call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Uuid,
    pub name: String,
    pub arguments: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            arguments,
        }
    }
}

/// Output from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_call_id: Uuid,
    pub content: String,
    pub is_error: bool,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl ToolOutput {
    pub fn success(tool_call_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            tool_call_id,
            content: content.into(),
            is_error: false,
            metadata: None,
        }
    }

    pub fn error(tool_call_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            tool_call_id,
            content: content.into(),
            is_error: true,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.metadata.is_none() {
            self.metadata = Some(std::collections::HashMap::new());
        }
        self.metadata.as_mut().unwrap().insert(key.into(), value.into());
        self
    }
}

/// Registry for managing available tools.
pub struct ToolRegistry {
    tools: parking_lot::RwLock<std::collections::HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn register<T: Tool + 'static>(&self, tool: T) {
        self.tools.write().insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn register_boxed(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().get(name).cloned()
    }

    pub fn get_with_metadata(&self, name: &str) -> Option<(Arc<dyn Tool>, ToolMetadata)> {
        self.tools
            .read()
            .get(name)
            .map(|t| (t.clone(), t.metadata()))
    }

    pub fn list(&self) -> Vec<String> {
        self.tools.read().keys().cloned().collect()
    }

    pub fn list_with_metadata(&self) -> Vec<ToolMetadata> {
        self.tools.read().values().map(|t| t.metadata()).collect()
    }

    pub fn remove(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.write().remove(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    pub fn count(&self) -> usize {
        self.tools.read().len()
    }

    pub fn execute(&self, call: ToolCall) -> impl std::future::Future<Output = Result<ToolOutput, ToolError>> + '_ {
        async move {
            let tool = self
                .get(&call.name)
                .ok_or_else(|| ToolError::NotFound(call.name.clone()))?;
            tool.execute(call.arguments).await
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 4 个通用工具 — 跨平台大脑层
// ============================================================================

/// Echo — 返回输入原样（用于 agent 自检、测试、占位）。
#[derive(Debug)]
pub struct EchoTool;

impl EchoTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo back the input text. Useful for testing and debugging."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to echo back" }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let msg = input
            .get("text")
            .or_else(|| input.get("message"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'text'".into()))?;
        Ok(ToolOutput::success(Uuid::new_v4(), msg.to_string()))
    }
}

/// Time — 返回当前时间（iso / unix / human 格式，支持 IANA `timezone`）。
#[derive(Debug)]
pub struct TimeTool;

impl TimeTool {
    pub fn new() -> Self {
        Self
    }

    fn get_timezone(&self, tz: Option<&str>) -> chrono_tz::Tz {
        tz.and_then(|s| s.parse().ok()).unwrap_or(chrono_tz::UTC)
    }
}

#[async_trait]
impl Tool for TimeTool {
    fn name(&self) -> &str {
        "time"
    }

    fn description(&self) -> &str {
        "Get the current time. Supports optional timezone."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "Optional timezone (e.g., 'America/New_York', 'Asia/Shanghai')"
                },
                "format": {
                    "type": "string",
                    "description": "Output format (default: 'iso')",
                    "enum": ["iso", "unix", "human"]
                }
            }
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        use chrono::Utc;
        let tz_name = input.get("timezone").and_then(|v| v.as_str());
        let format = input.get("format").and_then(|v| v.as_str()).unwrap_or("iso");
        let tz = self.get_timezone(tz_name);

        let now = Utc::now();
        let local = now.with_timezone(&tz);
        let output = match format {
            "unix" => now.timestamp().to_string(),
            "human" => local.format("%Y-%m-%d %H:%M:%S %Z").to_string(),
            _ => now.to_rfc3339(),
        };

        let details = serde_json::json!({
            "timezone": tz_name.unwrap_or("UTC"),
            "utc": now.to_rfc3339(),
            "unix": now.timestamp()
        });

        Ok(
            ToolOutput::success(Uuid::new_v4(), output)
                .with_metadata("details", details.to_string()),
        )
    }
}

/// JSON — 在 agent 上下文里格式化 / 解析 JSON（避免 LLM 直出大段结构化文本）。
#[derive(Debug)]
pub struct JsonTool;

impl JsonTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for JsonTool {
    fn name(&self) -> &str {
        "json"
    }

    fn description(&self) -> &str {
        "Format a JSON value with given indent, or extract a value at a JSON pointer."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "description": "JSON value (object/array/scalar)" },
                "pointer": { "type": "string", "description": "Optional JSON pointer (e.g. /foo/bar)" },
                "action": {
                    "type": "string",
                    "description": "Legacy: 'format' | 'minify' | 'validate' | 'parse' (acts on 'data')",
                    "enum": ["validate", "format", "minify", "parse"]
                },
                "data": { "type": "string", "description": "Legacy: JSON string to process with 'action'" }
            }
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        // Legacy shape {action, data} (prefrontal 迁移前) — 保持行为等价：
        // LLM prompt/历史 trace 里这两种调用都可能出现。
        if let Some(action) = input.get("action").and_then(|v| v.as_str()) {
            let data = input
                .get("data")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("missing 'data'".into()))?;
            let parsed: serde_json::Value = serde_json::from_str(data)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let out = match action {
                "validate" => "Valid JSON".to_string(),
                "format" | "parse" => serde_json::to_string_pretty(&parsed)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
                "minify" => serde_json::to_string(&parsed)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?,
                other => {
                    return Err(ToolError::InvalidParameters(format!(
                        "unknown action: {other}"
                    )));
                }
            };
            return Ok(ToolOutput::success(Uuid::new_v4(), out));
        }

        let value = input.get("value").cloned().unwrap_or(serde_json::Value::Null);
        if let Some(ptr) = input.get("pointer").and_then(|v| v.as_str()) {
            let extracted = value
                .pointer(ptr)
                .cloned()
                .ok_or_else(|| ToolError::NotFound(format!("pointer {ptr}")))?;
            let out = serde_json::to_string(&extracted)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            return Ok(ToolOutput::success(Uuid::new_v4(), out));
        }
        let out = serde_json::to_string_pretty(&value)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolOutput::success(Uuid::new_v4(), out))
    }
}

/// HTTP — 同步 HTTP 客户端（GET/POST/PUT/DELETE，body string）。
#[derive(Debug)]
pub struct HttpTool {
    client: reqwest::Client,
}

impl HttpTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn name(&self) -> &str {
        "http"
    }

    fn description(&self) -> &str {
        "Synchronous HTTP request (get/post/put/delete). Returns status + body (truncated to 8192 bytes)."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "method": { "type": "string", "enum": ["get", "post", "put", "delete"] },
                "url": { "type": "string" },
                "body": { "type": "string", "description": "Optional request body" },
                "headers": { "type": "object", "description": "Optional headers map" }
            },
            "required": ["method", "url"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let method = input
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'method'".into()))?;
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'url'".into()))?;
        let mut req = match method.to_lowercase().as_str() {
            "get" => self.client.get(url),
            "post" => self.client.post(url),
            "put" => self.client.put(url),
            "delete" => self.client.delete(url),
            other => {
                return Err(ToolError::InvalidParameters(format!(
                    "unsupported method: {other}"
                )));
            }
        };
        if let Some(body) = input.get("body").and_then(|v| v.as_str()) {
            req = req.body(body.to_string());
        }
        if let Some(headers) = input.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in headers {
                if let Some(s) = v.as_str() {
                    req = req.header(k.as_str(), s);
                }
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ToolError::ExternalServiceError(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| ToolError::ExternalServiceError(e.to_string()))?;
        let truncated = if body.len() > 8192 {
            format!("{}...<truncated {} bytes>", &body[..8192], body.len() - 8192)
        } else {
            body
        };
        Ok(ToolOutput::success(
            Uuid::new_v4(),
            format!("status: {status}\n\n{truncated}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "mock"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(&self, _: serde_json::Value) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::success(Uuid::new_v4(), "ok"))
        }
    }

    #[tokio::test]
    async fn echo_round_trip() {
        let out = EchoTool::new()
            .execute(serde_json::json!({"text": "hi"}))
            .await
            .unwrap();
        assert_eq!(out.content, "hi");
        assert!(!out.is_error);
    }

    #[tokio::test]
    async fn echo_accepts_message_alias() {
        let out = EchoTool::new()
            .execute(serde_json::json!({"message": "hi2"}))
            .await
            .unwrap();
        assert_eq!(out.content, "hi2");
    }

    #[tokio::test]
    async fn time_iso_format() {
        let out = TimeTool::new()
            .execute(serde_json::json!({"format": "iso"}))
            .await
            .unwrap();
        // 2026-10-15 prefix — RFC3339
        assert!(out.content.starts_with("20"), "iso: {}", out.content);
    }

    #[tokio::test]
    async fn time_unix_format() {
        let out = TimeTool::new()
            .execute(serde_json::json!({"format": "unix"}))
            .await
            .unwrap();
        assert!(out.content.parse::<i64>().is_ok(), "unix: {}", out.content);
    }

    #[tokio::test]
    async fn time_human_honors_timezone() {
        let out = TimeTool::new()
            .execute(serde_json::json!({"format": "human", "timezone": "Asia/Shanghai"}))
            .await
            .unwrap();
        assert!(out.content.ends_with("CST"), "human: {}", out.content);
        let details = out.metadata.as_ref().expect("details metadata");
        assert!(details.get("details").unwrap().contains("Asia/Shanghai"));
    }

    #[tokio::test]
    async fn json_legacy_action_format() {
        let out = JsonTool::new()
            .execute(serde_json::json!({"action": "format", "data": "{\"b\":2,\"a\":1}"}))
            .await
            .unwrap();
        assert!(out.content.contains("\"a\": 1"), "format: {}", out.content);
    }

    #[tokio::test]
    async fn json_legacy_action_minify_and_validate() {
        let min = JsonTool::new()
            .execute(serde_json::json!({"action": "minify", "data": "{ \"a\" : 1 }"}))
            .await
            .unwrap();
        assert_eq!(min.content, "{\"a\":1}");
        let ok = JsonTool::new()
            .execute(serde_json::json!({"action": "validate", "data": "[1,2]"}))
            .await
            .unwrap();
        assert_eq!(ok.content, "Valid JSON");
        let bad = JsonTool::new()
            .execute(serde_json::json!({"action": "validate", "data": "{oops"}))
            .await;
        assert!(bad.is_err());
    }

    #[tokio::test]
    async fn json_pointer_extract() {
        let out = JsonTool::new()
            .execute(serde_json::json!({"value": {"a": 42}, "pointer": "/a"}))
            .await
            .unwrap();
        assert_eq!(out.content, "42");
    }

    #[tokio::test]
    async fn json_pretty_print_default() {
        let out = JsonTool::new()
            .execute(serde_json::json!({"value": {"a": 1, "b": 2}}))
            .await
            .unwrap();
        assert!(out.content.contains("\"a\": 1"), "pretty: {}", out.content);
        assert!(out.content.contains("\"b\": 2"));
    }

    #[tokio::test]
    async fn http_invalid_method_errors() {
        let out = HttpTool::new()
            .execute(serde_json::json!({"method": "frob", "url": "http://x"}))
            .await;
        assert!(out.is_err());
        assert!(matches!(out.unwrap_err(), ToolError::InvalidParameters(_)));
    }

    #[tokio::test]
    async fn registry_register_and_execute() {
        let r = ToolRegistry::new();
        r.register(EchoTool::new());
        assert!(r.has("echo"));
        assert_eq!(r.count(), 1);
        let out = r
            .execute(ToolCall::new("echo", serde_json::json!({"text": "ok"})))
            .await
            .unwrap();
        assert_eq!(out.content, "ok");
    }

    #[tokio::test]
    async fn registry_not_found() {
        let r = ToolRegistry::new();
        let out = r
            .execute(ToolCall::new("nope", serde_json::json!({})))
            .await;
        assert!(matches!(out, Err(ToolError::NotFound(_))));
    }
}
