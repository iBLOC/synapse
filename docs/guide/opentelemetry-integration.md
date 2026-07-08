# OpenTelemetry 集成指南

本文档说明如何在 Synapse 中启用和使用 OpenTelemetry 集成。

## 概述

Synapse 通过可选的 `opentelemetry` feature 提供 OpenTelemetry 集成。默认不启用此 feature，
确保不引入额外的依赖。当启用时，Synapse 的 `observability` 模块可以与 OTel 集成，
将 Span 数据导出到任意 OTel 后端。

## 启用 Feature

在 `Cargo.toml` 中启用 feature：

```toml
[dependencies]
synapse = { version = "0.1", features = ["opentelemetry"] }
```

或通过命令行：

```bash
cargo build --features opentelemetry
cargo test --features opentelemetry
```

## 基本使用

### 1. 初始化 OTLP Exporter

```rust
use synapse::callbacks::{
    init_otel_provider, OtelSpanExporter,
};
use synapse::callbacks::observability::Tracer;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 OTLP exporter
    let provider = init_otel_provider(
        "my-synapse-agent",              // service.name
        "http://localhost:4317",        // OTLP gRPC endpoint
    )?;

    // 创建 OTel Span Exporter
    let otel_exporter = OtelSpanExporter::new(&provider, "my-synapse-agent");

    // 添加到 Synapse Tracer
    let tracer = Tracer::new().with_exporter(Arc::new(otel_exporter));

    // 使用 tracer 追踪操作
    let mut span = tracer.start_span(
        "my_operation",
        ComponentType::Agent,
        "agent1",
        RunType::Invoke,
    );
    span.set_input(serde_json::json!({"query": "hello"}));
    
    // ... 执行操作 ...
    
    span.set_output(serde_json::json!({"result": "world"}));
    let _ = span.finish();

    Ok(())
}
```

### 2. 使用全局 Provider

```rust
use synapse::callbacks::{set_global_provider, OtelSpanExporter};

let provider = init_otel_provider("service", "http://localhost:4317")?;
set_global_provider(provider);

// 之后所有 OtelSpanExporter::new 都会使用这个全局 provider
```

### 3. 自定义资源属性

```rust
use std::collections::HashMap;

let mut attrs = HashMap::new();
attrs.insert("deployment.environment".into(), "production".into());
attrs.insert("service.version".into(), "1.2.3".into());

let provider = init_otel_provider_with_attrs(
    "my-service",
    "http://localhost:4317",
    attrs,
)?;
```

### 4. 集成到 Callback 系统

```rust
use synapse::callbacks::{
    CallbackManager, TracerHandler,
};
use std::sync::Arc;

// 创建带 OTel 追踪的 Handler
let tracer = Arc::new(Tracer::new()
    .with_exporter(Arc::new(otel_exporter)));
let handler: Arc<dyn Handler> = Arc::new(TracerHandler::new(tracer));

// 添加到 callback manager
let manager = CallbackManager::new().with_handler(handler);

// 在 Context 中使用
let ctx = Context::new().with_callback_manager(manager);
```

## 配置示例

### Docker Compose（本地开发）

```yaml
version: '3'
services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC
      - "4318:4318"    # OTLP HTTP

  my-app:
    build: .
    environment:
      - OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
```

### Tempo（Grafana）

```bash
# docker-compose.yml
version: '3'
services:
  tempo:
    image: grafana/tempo:latest
    command: ["-config.file=/etc/tempo.yaml"]
    volumes:
      - ./tempo.yaml:/etc/tempo.yaml
    ports:
      - "4317:4317"

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
```

## 数据映射

Synapse Span 字段映射到 OTel 属性：

| Synapse 字段 | OTel 属性 | 说明 |
|--------------|-----------|------|
| `span.id` | `synapse.span.id` | Span 唯一 ID |
| `span.trace_id` | `synapse.trace.id` | Trace ID |
| `span.parent_id` | `synapse.parent_id` | 父 Span ID |
| `span.component` | `synapse.component` | 组件类型 |
| `span.component_name` | `synapse.component_name` | 组件名 |
| `span.run_type` | `synapse.run_type` | 运行类型 |
| `span.duration()` | `synapse.duration_ns` | 持续时间（纳秒） |
| `span.error` | `synapse.error` | 错误信息 |
| `span.attributes` | `synapse.attr.*` | 自定义属性 |
| `span.input` | `synapse.input` | 输入（截断到 1KB） |
| `span.output` | `synapse.output` | 输出（截断到 1KB） |
| `span.status` | OTel Span Status | Ok/Error |

## 性能考虑

- **零开销**：默认不启用 OTel feature
- **批处理**：OTel 自动批量发送 Span，降低网络开销
- **异步导出**：不阻塞主调用流程
- **内存开销**：约 200-500 bytes/span

### 推荐配置

```rust
// 生产环境：使用批处理（默认）
let provider = init_otel_provider("service", "endpoint")?;

// 高 QPS 场景：调整批处理参数
use opentelemetry_sdk::trace::BatchConfig;
use std::time::Duration;

let batch_config = BatchConfig::builder()
    .with_max_queue_size(2048)
    .with_max_export_batch_size(512)
    .with_scheduled_delay(Duration::from_secs(5))
    .build();
```

## 故障排查

### 连接失败

```
Error: failed to build exporter: transport error
```

检查 OTLP endpoint 是否可访问。Telnet 测试：
```bash
nc -zv localhost 4317
```

### Span 未导出

1. 确认 `finish()` 被调用
2. 确认 `provider.force_flush()` 在程序退出前被调用
3. 检查采样率配置

### 性能问题

1. 减少 `set_input`/`set_output` 的数据量（自动截断到 1KB）
2. 启用采样（仅追踪 1% 的请求）
3. 增加批处理延迟

## 与 W3C Trace Context 集成

OTel 支持 W3C Trace Context 标准的 trace 传播。当前实现中，
Synapse 自动生成 trace_id，但如果你需要在跨服务调用中传播 context，
可以使用以下模式：

```rust
// 提取上游 trace context
use opentelemetry::global;
use opentelemetry::propagation::Extractor;

let carrier = HashMap::new(); // 从 HTTP headers 提取
let parent_cx = global::get_text_map_propagator(|prop| prop.extract(&carrier));

// 创建子 span
let span = tracer.span_builder("child_op")
    .with_kind(SpanKind::Internal)
    .start_with_context(&tracer, &parent_cx);
```

## 参考

- [OpenTelemetry Rust 文档](https://docs.rs/opentelemetry/)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/otel/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [评估文档](./opentelemetry-evaluation.md) - 详细的集成评估和设计决策