//! 增强的可观测性（BeeAI 风格）
//!
//! 提供 Span/Trace 支持，追踪每次运行的耗时、输入输出、错误等。
//! 类似 BeeAI 的可观测性设计和 OpenTelemetry 的 trace 模型。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{CallbackError, CallbackManager, CallbackResult, ComponentType, Handler, RunInfo, RunType};

/// Span 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    Running,
    Ok,
    Error,
}

/// Span 状态追踪器
#[derive(Clone)]
pub struct Span {
    pub id: String,
    pub parent_id: Option<String>,
    pub trace_id: String,
    pub name: String,
    pub component: ComponentType,
    pub component_name: String,
    pub run_type: RunType,
    pub started_at: Instant,
    pub ended_at: Option<Instant>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, serde_json::Value>,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub children: Vec<String>,
}

impl Span {
    pub fn new(
        parent_id: Option<String>,
        trace_id: String,
        name: String,
        component: ComponentType,
        component_name: String,
        run_type: RunType,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            trace_id,
            name,
            component,
            component_name,
            run_type,
            started_at: Instant::now(),
            ended_at: None,
            status: SpanStatus::Running,
            attributes: HashMap::new(),
            input: None,
            output: None,
            error: None,
            children: Vec::new(),
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.ended_at.map(|e| e.duration_since(self.started_at))
    }

    pub fn set_input(&mut self, input: serde_json::Value) {
        self.input = Some(input);
    }

    pub fn set_output(&mut self, output: serde_json::Value) {
        self.output = Some(output);
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.status = SpanStatus::Error;
    }

    pub fn add_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }

    pub fn finish(&mut self) {
        self.ended_at = Some(Instant::now());
        if self.status == SpanStatus::Running {
            self.status = SpanStatus::Ok;
        }
    }
}

/// Span 导出器
pub trait SpanExporter: Send + Sync {
    fn export(&self, span: &Span);
}

/// 标准输出导出器 - 用于调试
pub struct StdoutExporter;

impl SpanExporter for StdoutExporter {
    fn export(&self, span: &Span) {
        let duration_ms = span.duration().map(|d| d.as_millis()).unwrap_or(0);
        eprintln!(
            "[span] id={} name={} component={:?} status={:?} duration={}ms",
            span.id, span.name, span.component, span.status, duration_ms
        );
    }
}

/// 内存导出器 - 用于测试和聚合
#[derive(Clone)]
pub struct MemoryExporter {
    spans: Arc<RwLock<Vec<Span>>>,
}

impl MemoryExporter {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_spans(&self) -> Vec<Span> {
        self.spans.read().await.to_vec()
    }

    pub async fn clear(&self) {
        self.spans.write().await.clear();
    }
}

impl Default for MemoryExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanExporter for MemoryExporter {
    fn export(&self, span: &Span) {
        let span = span.clone();
        let spans = self.spans.clone();
        // Try to push synchronously if possible
        if let Ok(mut guard) = self.spans.try_write() {
            guard.push(span);
        } else {
            // Fallback: spawn async push
            tokio::spawn(async move {
                spans.write().await.push(span);
            });
        }
    }
}

/// Tracer - 创建 Span 的工厂
pub struct Tracer {
    exporters: Vec<Arc<dyn SpanExporter>>,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            exporters: Vec::new(),
        }
    }

    pub fn with_exporter(mut self, exporter: Arc<dyn SpanExporter>) -> Self {
        self.exporters.push(exporter);
        self
    }

    pub fn start_span(
        &self,
        name: &str,
        component: ComponentType,
        component_name: &str,
        run_type: RunType,
    ) -> ActiveSpan {
        let span = Span::new(
            None,
            Uuid::new_v4().to_string(),
            name.to_string(),
            component,
            component_name.to_string(),
            run_type,
        );
        ActiveSpan {
            span,
            exporters: self.exporters.clone(),
        }
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}

/// 活跃的 Span - 在作用域结束时自动结束
pub struct ActiveSpan {
    span: Span,
    exporters: Vec<Arc<dyn SpanExporter>>,
}

impl ActiveSpan {
    pub fn child(&self, name: &str, component: ComponentType, component_name: &str, run_type: RunType) -> ActiveSpan {
        let span = Span::new(
            Some(self.span.id.clone()),
            self.span.trace_id.clone(),
            name.to_string(),
            component,
            component_name.to_string(),
            run_type,
        );
        ActiveSpan {
            span,
            exporters: self.exporters.clone(),
        }
    }

    pub fn set_input(&mut self, input: serde_json::Value) {
        self.span.set_input(input);
    }

    pub fn set_output(&mut self, output: serde_json::Value) {
        self.span.set_output(output);
    }

    pub fn set_error(&mut self, error: String) {
        self.span.set_error(error);
    }

    pub fn add_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.span.add_attribute(key, value);
    }

    pub fn id(&self) -> &str {
        &self.span.id
    }

    pub fn finish(self) -> Span {
        let mut span = self.span;
        span.finish();
        for exporter in &self.exporters {
            exporter.export(&span);
        }
        span
    }
}

/// Tracer Handler - 将 tracer 集成到现有的 callback 系统
pub struct TracerHandler {
    tracer: Arc<Tracer>,
    active_spans: Arc<RwLock<HashMap<String, ActiveSpan>>>,
}

impl TracerHandler {
    pub fn new(tracer: Arc<Tracer>) -> Self {
        Self {
            tracer,
            active_spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Handler for TracerHandler {
    fn on_start(&self, info: &RunInfo, input: &serde_json::Value) {
        let active = self.tracer.start_span(
            &info.name,
            info.component,
            &info.component_name,
            info.run_type,
        );
        let mut active = active;
        active.set_input(input.clone());
        let id = active.id().to_string();
        let id_clone = id.clone();
        let active_spans = self.active_spans.clone();
        tokio::spawn(async move {
            active_spans.write().await.insert(id_clone, active);
        });
    }

    fn on_end(&self, info: &RunInfo, output: &serde_json::Value) {
        let active_spans = self.active_spans.clone();
        let component_name = info.component_name.clone();
        let output = output.clone();
        tokio::spawn(async move {
            let mut spans = active_spans.write().await;
            // Find and finish any span for this component
            let keys: Vec<String> = spans
                .keys()
                .filter(|k| {
                    spans
                        .get(*k)
                        .map(|s| s.span.component_name == component_name)
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            for key in keys {
                if let Some(mut active) = spans.remove(&key) {
                    active.set_output(output.clone());
                    active.finish();
                }
            }
        });
    }

    fn on_error(&self, info: &RunInfo, error: &str) {
        let active_spans = self.active_spans.clone();
        let component_name = info.component_name.clone();
        let error = error.to_string();
        tokio::spawn(async move {
            let mut spans = active_spans.write().await;
            let keys: Vec<String> = spans
                .keys()
                .filter(|k| {
                    spans
                        .get(*k)
                        .map(|s| s.span.component_name == component_name)
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            for key in keys {
                if let Some(mut active) = spans.remove(&key) {
                    active.set_error(error.clone());
                    active.finish();
                }
            }
        });
    }
}

/// 全局指标计数器
pub struct Metrics {
    pub invoke_count: AtomicU64,
    pub error_count: AtomicU64,
    pub total_duration_ms: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            invoke_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
        }
    }

    pub fn record_invoke(&self, duration: Duration) {
        self.invoke_count.fetch_add(1, Ordering::SeqCst);
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            invoke_count: self.invoke_count.load(Ordering::SeqCst),
            error_count: self.error_count.load(Ordering::SeqCst),
            total_duration_ms: self.total_duration_ms.load(Ordering::SeqCst),
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub invoke_count: u64,
    pub error_count: u64,
    pub total_duration_ms: u64,
}

/// Metrics Handler - 自动记录每次调用的指标
pub struct MetricsHandler {
    pub metrics: Arc<Metrics>,
}

impl MetricsHandler {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

impl Handler for MetricsHandler {
    fn on_start(&self, _info: &RunInfo, _input: &serde_json::Value) {
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            metrics.invoke_count.fetch_add(1, Ordering::SeqCst);
        });
    }

    fn on_error(&self, _info: &RunInfo, _error: &str) {
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            metrics.error_count.fetch_add(1, Ordering::SeqCst);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let mut span = Span::new(
            None,
            "trace-1".to_string(),
            "test_span".to_string(),
            ComponentType::ChatModel,
            "test_model".to_string(),
            RunType::Invoke,
        );
        assert_eq!(span.status, SpanStatus::Running);
        span.set_input(serde_json::json!({"input": "test"}));
        span.set_output(serde_json::json!({"output": "result"}));
        span.finish();
        assert_eq!(span.status, SpanStatus::Ok);
        assert!(span.duration().is_some());
    }

    #[test]
    fn test_span_error() {
        let mut span = Span::new(
            None,
            "trace-1".to_string(),
            "test_span".to_string(),
            ComponentType::Tool,
            "test_tool".to_string(),
            RunType::Invoke,
        );
        span.set_error("test error".to_string());
        assert_eq!(span.status, SpanStatus::Error);
        span.finish();
        assert_eq!(span.error, Some("test error".to_string()));
    }

    #[tokio::test]
    async fn test_memory_exporter() {
        let exporter = MemoryExporter::new();
        let tracer = Tracer::new().with_exporter(Arc::new(exporter.clone()) as Arc<dyn SpanExporter>);

        let span = tracer.start_span("test", ComponentType::ChatModel, "model", RunType::Invoke);
        let span = span.finish();

        exporter.export(&span);

        // Wait a bit for any async tasks
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let spans = exporter.get_spans().await;
        assert!(spans.len() >= 1);
    }

    #[tokio::test]
    async fn test_metrics() {
        let metrics = Arc::new(Metrics::new());
        metrics.record_invoke(Duration::from_millis(100));
        metrics.record_invoke(Duration::from_millis(200));
        metrics.record_error();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.invoke_count, 2);
        assert_eq!(snapshot.error_count, 1);
        assert_eq!(snapshot.total_duration_ms, 300);
    }

    #[tokio::test]
    async fn test_tracer_handler() {
        let tracer = Arc::new(Tracer::new());
        let _handler = TracerHandler::new(tracer);

        let info = RunInfo::new("test", RunType::Invoke, ComponentType::ChatModel, "model");
        let handler = TracerHandler::new(Arc::new(Tracer::new()));
        handler.on_start(&info, &serde_json::json!({}));
    }
}