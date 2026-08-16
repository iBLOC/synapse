//! OpenTelemetry 集成（feature-gated）
//!
//! 提供 Synapse observability 模块与 OpenTelemetry 的桥接。
//! 默认不启用，需要 `opentelemetry` feature。
//!
//! # 使用示例
//!
//! ```ignore
//! use synapse::callbacks::opentelemetry_exporter::{OtelSpanExporter, init_otel_provider};
//!
//! // 初始化 OTLP exporter
//! let provider = init_otel_provider("synapse-agent", "http://localhost:4317")?;
//!
//! // 创建导出器
//! let exporter = OtelSpanExporter::new(provider.tracer("synapse"));
//!
//! // 添加到 Tracer
//! let tracer = Tracer::new().with_exporter(Arc::new(exporter));
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use opentelemetry::trace::{Span as OtelSpanTrait, SpanKind, Status, Tracer as OtelTracerTrait};
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;

use crate::callbacks::observability::{Span, SpanExporter, SpanStatus};

/// OpenTelemetry Span Exporter
///
/// 将 Synapse 的 Span 数据转换为 OpenTelemetry 格式并导出。
pub struct OtelSpanExporter {
    tracer: Arc<dyn OtelTracerTrait + Send + Sync>,
    service_name: String,
}

impl OtelSpanExporter {
    /// 从 SdkTracerProvider 创建导出器
    pub fn new(provider: &SdkTracerProvider, service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        let tracer = provider.tracer(service_name.clone());
        Self {
            tracer: Arc::new(tracer),
            service_name,
        }
    }

    /// 从全局 tracer 创建导出器
    pub fn from_global(service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        let tracer = opentelemetry::global::tracer(service_name.clone());
        Self {
            tracer: Arc::new(tracer),
            service_name,
        }
    }

    /// 强制刷新待处理的 spans
    pub fn flush(&self) {
        // SdkTracerProvider 的 flush 需要单独调用
    }
}

impl SpanExporter for OtelSpanExporter {
    fn export(&self, span: &Span) {
        let duration_ns = span
            .duration()
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        // 构建属性
        let mut attributes: Vec<KeyValue> = Vec::new();
        attributes.push(KeyValue::new("synapse.span.id", span.id.clone()));
        attributes.push(KeyValue::new("synapse.trace.id", span.trace_id.clone()));
        attributes.push(KeyValue::new(
            "synapse.component",
            format!("{:?}", span.component),
        ));
        attributes.push(KeyValue::new(
            "synapse.component_name",
            span.component_name.clone(),
        ));
        attributes.push(KeyValue::new(
            "synapse.run_type",
            format!("{:?}", span.run_type),
        ));
        attributes.push(KeyValue::new("synapse.duration_ns", duration_ns as i64));

        if let Some(parent_id) = &span.parent_id {
            attributes.push(KeyValue::new("synapse.parent_id", parent_id.clone()));
        }

        // 错误信息
        if let Some(error) = &span.error {
            attributes.push(KeyValue::new("synapse.error", error.clone()));
        }

        // 自定义属性
        for (key, value) in &span.attributes {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            attributes.push(KeyValue::new(format!("synapse.attr.{}", key), value_str));
        }

        // 输入输出（限制大小避免过大）
        if let Some(input) = &span.input {
            let input_str = input.to_string();
            let truncated = if input_str.len() > 1024 {
                format!("{}...", &input_str[..1024])
            } else {
                input_str
            };
            attributes.push(KeyValue::new("synapse.input", truncated));
        }

        if let Some(output) = &span.output {
            let output_str = output.to_string();
            let truncated = if output_str.len() > 1024 {
                format!("{}...", &output_str[..1024])
            } else {
                output_str
            };
            attributes.push(KeyValue::new("synapse.output", truncated));
        }

        // 创建 OTel span
        let kind = match span.run_type {
            crate::callbacks::RunType::Invoke => SpanKind::Internal,
            crate::callbacks::RunType::Stream => SpanKind::Internal,
            crate::callbacks::RunType::Collect => SpanKind::Internal,
            crate::callbacks::RunType::Transform => SpanKind::Internal,
        };

        let mut otel_span = self
            .tracer
            .span_builder(span.name.clone())
            .with_kind(kind)
            .with_attributes(attributes)
            .with_start_time(span.started_at)
            .start(self.tracer.as_ref());

        // 设置状态
        match span.status {
            SpanStatus::Ok => {
                otel_span.set_status(Status::Ok);
            }
            SpanStatus::Error => {
                let description = span.error.clone().unwrap_or_else(|| "unknown error".into());
                otel_span.set_status(Status::error(description));
            }
            SpanStatus::Running => {}
        }

        // 结束 span
        if let Some(ended_at) = span.ended_at {
            otel_span.end_with_timestamp(ended_at);
        } else {
            otel_span.end();
        }
    }
}

/// 初始化 OpenTelemetry TracerProvider
///
/// 通过 OTLP gRPC 协议连接到 collector。
pub fn init_otel_provider(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<SdkTracerProvider, OtelError> {
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::Config;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .map_err(|e| OtelError::ExporterBuild(e.to_string()))?;

    let resource = Resource::builder()
        .with_attributes([KeyValue::new("service.name", service_name.to_string())])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    Ok(provider)
}

/// 初始化带自定义属性的 OpenTelemetry TracerProvider
pub fn init_otel_provider_with_attrs(
    service_name: &str,
    otlp_endpoint: &str,
    additional_attrs: HashMap<String, String>,
) -> Result<SdkTracerProvider, OtelError> {
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .map_err(|e| OtelError::ExporterBuild(e.to_string()))?;

    let mut attrs = vec![KeyValue::new("service.name", service_name.to_string())];
    for (k, v) in additional_attrs {
        attrs.push(KeyValue::new(k, v));
    }

    let resource = Resource::builder().with_attributes(attrs).build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    Ok(provider)
}

/// 设置全局 tracer provider
pub fn set_global_provider(provider: SdkTracerProvider) {
    opentelemetry::global::set_tracer_provider(provider);
}

/// OpenTelemetry 集成错误
#[derive(Debug, thiserror::Error)]
pub enum OtelError {
    #[error("failed to build exporter: {0}")]
    ExporterBuild(String),

    #[error("provider error: {0}")]
    Provider(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callbacks::observability::Tracer;
    use crate::callbacks::{ComponentType, RunInfo, RunType};
    use std::time::Duration;

    fn create_test_span() -> Span {
        let mut span = Span::new(
            None,
            "trace-1".to_string(),
            "test_op".to_string(),
            ComponentType::ChatModel,
            "gpt-4".to_string(),
            RunType::Invoke,
        );
        span.set_input(serde_json::json!({"query": "hello"}));
        span.set_output(serde_json::json!({"response": "world"}));
        span.set_attribute("user_id", serde_json::json!("alice"));
        span.finish();
        span
    }

    #[test]
    fn test_otel_span_exporter_creation() {
        let provider = SdkTracerProvider::builder().build();
        let exporter = OtelSpanExporter::new(&provider, "test-service");
        assert_eq!(exporter.service_name, "test-service");
    }

    #[test]
    fn test_otel_span_exporter_export() {
        let provider = SdkTracerProvider::builder().build();
        let exporter = OtelSpanExporter::new(&provider, "test-service");

        let span = create_test_span();
        // 验证 export 不会 panic
        exporter.export(&span);
    }

    #[test]
    fn test_otel_span_with_error() {
        let provider = SdkTracerProvider::builder().build();
        let exporter = OtelSpanExporter::new(&provider, "test-service");

        let mut span = Span::new(
            None,
            "trace-1".to_string(),
            "failing_op".to_string(),
            ComponentType::Tool,
            "my_tool".to_string(),
            RunType::Invoke,
        );
        span.set_error("connection timeout".to_string());
        span.finish();

        exporter.export(&span);
    }

    #[test]
    fn test_otel_exporter_with_tracer() {
        let provider = SdkTracerProvider::builder().build();
        let otel_exporter = OtelSpanExporter::new(&provider, "test-service");
        let tracer = Tracer::new().with_exporter(Arc::new(otel_exporter));

        let span = tracer.start_span(
            "test",
            ComponentType::ChatModel,
            "model",
            RunType::Invoke,
        );
        let span = span.finish();
        // 验证完整流程不 panic
        std::thread::sleep(Duration::from_millis(10));
    }
}