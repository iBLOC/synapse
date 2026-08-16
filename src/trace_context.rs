//! W3C Trace Context 传播（feature-gated）
//!
//! 实现 W3C Trace Context 标准 (https://www.w3.org/TR/trace-context/)，
//! 用于跨服务传播 trace 和 span ID。
//!
//! 默认不启用，需要 `opentelemetry` feature。
//!
//! # 使用示例
//!
//! ```ignore
//! use synapse::trace_context::{HeaderInjector, HeaderExtractor};
//!
//! // 注入到 outgoing 请求
//! let mut injector = HeaderInjector::new();
//! global_propagator.inject_context(&cx, &mut injector);
//! let headers = injector.into_headers();
//!
//! // 从 incoming 请求提取
//! let mut extractor = HeaderExtractor::from_headers(&headers);
//! let cx = global_propagator.extract(&extractor);
//! ```

use std::collections::HashMap;

/// W3C Trace Context header 名称
pub const TRACEPARENT: &str = "traceparent";
pub const TRACESTATE: &str = "tracestate";

/// TraceParent 格式: `version-trace_id-parent_id-trace_flags`
///
/// 示例: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
#[derive(Debug, Clone, PartialEq)]
pub struct TraceParent {
    /// 版本号（当前为 "00"）
    pub version: String,
    /// Trace ID (32 hex chars, 16 bytes)
    pub trace_id: String,
    /// Parent Span ID (16 hex chars, 8 bytes)
    pub span_id: String,
    /// Trace flags (1 byte)
    pub flags: String,
}

impl TraceParent {
    /// 从 header 值解析
    pub fn parse(value: &str) -> Result<Self, TraceContextError> {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return Err(TraceContextError::InvalidFormat(format!(
                "expected 4 parts, got {}",
                parts.len()
            )));
        }

        let version = parts[0].to_string();
        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        let flags = parts[3].to_string();

        // 验证格式
        if version.len() != 2 {
            return Err(TraceContextError::InvalidFormat(
                "version must be 2 hex chars".into(),
            ));
        }
        if trace_id.len() != 32 || !trace_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TraceContextError::InvalidTraceId(trace_id));
        }
        if span_id.len() != 16 || !span_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TraceContextError::InvalidSpanId(span_id));
        }
        if flags.len() != 2 || !flags.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TraceContextError::InvalidFlags(flags));
        }

        // 验证 trace_id 和 span_id 不全为 0
        if trace_id == "00000000000000000000000000000000" {
            return Err(TraceContextError::InvalidTraceId(
                "trace_id cannot be all zeros".into(),
            ));
        }
        if span_id == "0000000000000000" {
            return Err(TraceContextError::InvalidSpanId(
                "span_id cannot be all zeros".into(),
            ));
        }

        Ok(Self {
            version,
            trace_id,
            span_id,
            flags,
        })
    }

    /// 序列化为 header 值
    pub fn to_string(&self) -> String {
        format!("{}-{}-{}-{}", self.version, self.trace_id, self.span_id, self.flags)
    }
}

/// Trace Context 错误
#[derive(Debug, thiserror::Error)]
pub enum TraceContextError {
    #[error("invalid trace context format: {0}")]
    InvalidFormat(String),

    #[error("invalid trace_id: {0}")]
    InvalidTraceId(String),

    #[error("invalid span_id: {0}")]
    InvalidSpanId(String),

    #[error("invalid trace flags: {0}")]
    InvalidFlags(String),
}

/// HTTP Header Extractor
///
/// 从 HTTP headers 中提取 W3C Trace Context。
#[derive(Debug, Clone, Default)]
pub struct HeaderExtractor {
    headers: HashMap<String, String>,
}

impl HeaderExtractor {
    /// 从 HashMap 创建 extractor
    pub fn new(headers: HashMap<String, String>) -> Self {
        let mut normalized = HashMap::new();
        for (k, v) in headers {
            normalized.insert(k.to_lowercase(), v);
        }
        Self { headers: normalized }
    }

    /// 从 Vec<(String, String)> 创建 extractor
    pub fn from_headers(headers: &[(String, String)]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in headers {
            map.insert(k.to_lowercase(), v.clone());
        }
        Self { headers: map }
    }

    /// 获取 header 值
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(String::as_str)
    }

    /// 提取 TraceParent
    pub fn traceparent(&self) -> Result<Option<TraceParent>, TraceContextError> {
        match self.get(TRACEPARENT) {
            Some(value) => TraceParent::parse(value).map(Some),
            None => Ok(None),
        }
    }

    /// 提取 TraceState
    pub fn tracestate(&self) -> Option<String> {
        self.get(TRACESTATE).map(String::from)
    }

    /// 提取的 keys
    pub fn keys(&self) -> Vec<String> {
        self.headers.keys().cloned().collect()
    }
}

/// HTTP Header Injector
///
/// 用于将 W3C Trace Context 注入到 outgoing HTTP 请求。
#[derive(Debug, Clone, Default)]
pub struct HeaderInjector {
    headers: HashMap<String, String>,
}

impl HeaderInjector {
    /// 创建空的 injector
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 header
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(key.into().to_lowercase(), value.into());
    }

    /// 注入 traceparent
    pub fn set_traceparent(&mut self, traceparent: &TraceParent) {
        self.set(TRACEPARENT, traceparent.to_string());
    }

    /// 注入 tracestate
    pub fn set_tracestate(&mut self, tracestate: impl Into<String>) {
        self.set(TRACESTATE, tracestate.into());
    }

    /// 获取 header
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(String::as_str)
    }

    /// 转换为 Vec<(String, String)>
    pub fn into_headers(self) -> Vec<(String, String)> {
        self.headers.into_iter().collect()
    }

    /// 转换为 HashMap
    pub fn into_map(self) -> HashMap<String, String> {
        self.headers
    }
}

/// 创建新的 traceparent
pub fn create_traceparent(trace_id: &str, span_id: &str) -> TraceParent {
    TraceParent {
        version: "00".to_string(),
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        flags: "01".to_string(), // sampled
    }
}

/// 生成新的 32-char trace ID
pub fn generate_trace_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    // 简单的 trace ID 生成（基于时间戳 + 随机数）
    let timestamp = format!("{:032x}", now);
    timestamp[timestamp.len() - 32..].to_string()
}

/// 生成新的 16-char span ID
pub fn generate_span_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let span_id = format!("{:016x}", now & 0xFFFFFFFFFFFFFFFF);
    span_id[span_id.len() - 16..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traceparent_parse() {
        let value = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let tp = TraceParent::parse(value).unwrap();
        assert_eq!(tp.version, "00");
        assert_eq!(tp.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(tp.span_id, "b7ad6b7169203331");
        assert_eq!(tp.flags, "01");
    }

    #[test]
    fn test_traceparent_to_string() {
        let tp = TraceParent {
            version: "00".to_string(),
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            flags: "01".to_string(),
        };
        assert_eq!(
            tp.to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
    }

    #[test]
    fn test_traceparent_round_trip() {
        let original = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let tp = TraceParent::parse(original).unwrap();
        assert_eq!(tp.to_string(), original);
    }

    #[test]
    fn test_traceparent_invalid_format() {
        let result = TraceParent::parse("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_traceparent_invalid_trace_id() {
        let result = TraceParent::parse("00-short-b7ad6b7169203331-01");
        assert!(matches!(
            result,
            Err(TraceContextError::InvalidTraceId(_))
        ));
    }

    #[test]
    fn test_traceparent_all_zero_trace_id() {
        let result = TraceParent::parse(
            "00-00000000000000000000000000000000-b7ad6b7169203331-01",
        );
        assert!(matches!(
            result,
            Err(TraceContextError::InvalidTraceId(_))
        ));
    }

    #[test]
    fn test_traceparent_all_zero_span_id() {
        let result = TraceParent::parse(
            "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01",
        );
        assert!(matches!(
            result,
            Err(TraceContextError::InvalidSpanId(_))
        ));
    }

    #[test]
    fn test_header_extractor() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );
        headers.insert("content-type".to_string(), "application/json".to_string());

        let extractor = HeaderExtractor::new(headers);
        let tp = extractor.traceparent().unwrap().unwrap();
        assert_eq!(tp.trace_id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_header_extractor_case_insensitive() {
        let mut headers = HashMap::new();
        headers.insert(
            "TraceParent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let extractor = HeaderExtractor::new(headers);
        assert!(extractor.traceparent().unwrap().is_some());
    }

    #[test]
    fn test_header_injector() {
        let mut injector = HeaderInjector::new();
        let tp = TraceParent::parse(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        )
        .unwrap();
        injector.set_traceparent(&tp);

        assert!(injector.get("traceparent").is_some());
        let headers = injector.into_headers();
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn test_generate_trace_id() {
        let id = generate_trace_id();
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_span_id() {
        let id = generate_span_id();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_create_traceparent() {
        let tp = create_traceparent(
            "0af7651916cd43dd8448eb211c80319c",
            "b7ad6b7169203331",
        );
        assert_eq!(tp.version, "00");
        assert_eq!(tp.flags, "01");
    }

    #[test]
    fn test_extractor_from_vec() {
        let headers = vec![
            (
                "traceparent".to_string(),
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ];
        let extractor = HeaderExtractor::from_headers(&headers);
        assert!(extractor.traceparent().unwrap().is_some());
        assert_eq!(extractor.get("content-type"), Some("application/json"));
    }
}