# 增强可观测性（BeeAI 风格）

`synapse::callbacks::observability` 模块提供了类似 BeeAI Framework 的 Span/Trace 可观测性能力。

## 概述

BeeAI Framework（IBM 开源的企业级 Agent 框架）强调可观测性和生产部署能力。
Synapse 的 `observability` 模块对齐了这一设计，提供：

- **Span/Trace** 追踪每次运行的耗时、输入输出、错误
- **可插拔导出器** 支持标准输出、内存、第三方（如 OpenTelemetry）
- **指标收集** 自动记录调用次数、错误数、耗时

## 核心类型

### Span

```rust
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
```

Span 状态：

```rust
pub enum SpanStatus {
    Running,
    Ok,
    Error,
}
```

### Tracer

```rust
pub struct Tracer {
    exporters: Vec<Arc<dyn SpanExporter>>,
}
```

Tracer 是创建 Span 的工厂，支持挂载多个导出器。

### ActiveSpan

```rust
pub struct ActiveSpan {
    span: Span,
    exporters: Vec<Arc<dyn SpanExporter>>,
}
```

活跃的 Span - 在作用域结束时自动结束（通过 `finish()`）。

### SpanExporter

```rust
pub trait SpanExporter: Send + Sync {
    fn export(&self, span: &Span);
}
```

可插拔的 Span 导出器接口。

内置实现：
- `StdoutExporter` — 标准输出导出器（用于调试）
- `MemoryExporter` — 内存导出器（用于测试和聚合）

### Metrics

```rust
pub struct Metrics {
    pub invoke_count: AtomicU64,
    pub error_count: AtomicU64,
    pub total_duration_ms: AtomicU64,
}
```

全局指标计数器。

## 使用示例

### 基本 Span 追踪

```rust
use synapse::callbacks::observability::*;

let tracer = Tracer::new();

let mut span = tracer.start_span(
    "my_operation",
    ComponentType::ChatModel,
    "gpt-4",
    RunType::Invoke,
);

span.set_input(serde_json::json!({"query": "hello"}));
// ... 执行操作
span.set_output(serde_json::json!({"response": "world"}));

let final_span = span.finish();
// 自动导出到所有注册的 exporter
```

### 使用内存导出器

```rust
let exporter = Arc::new(MemoryExporter::new());
let tracer = Tracer::new()
    .with_exporter(exporter.clone() as Arc<dyn SpanExporter>);

let span = tracer.start_span("test", ComponentType::Tool, "my_tool", RunType::Invoke);
let _ = span.finish();

// 检查收集到的 Span
let spans = exporter.get_spans().await;
assert_eq!(spans.len(), 1);
```

### 集成到现有 Callback 系统

```rust
use synapse::callbacks::{TracerHandler, CallbackManager};

let tracer = Arc::new(Tracer::new());
let handler: Arc<dyn Handler> = Arc::new(TracerHandler::new(tracer));

let manager = CallbackManager::new().with_handler(handler);
// 现在所有 on_start/on_end/on_error 都会自动追踪
```

### 指标收集

```rust
let metrics = Arc::new(Metrics::new());
let handler: Arc<dyn Handler> = Arc::new(MetricsHandler::new(metrics.clone()));

// ... 在 handler 触发后 ...
let snapshot = metrics.snapshot();
println!("调用次数: {}", snapshot.invoke_count);
println!("错误次数: {}", snapshot.error_count);
println!("总耗时(ms): {}", snapshot.total_duration_ms);
```

### 自定义导出器

实现 `SpanExporter` trait 即可创建自定义导出器：

```rust
struct OtelExporter;

impl SpanExporter for OtelExporter {
    fn export(&self, span: &Span) {
        // 将 Span 转换为 OpenTelemetry 格式并发送
        // ...
    }
}

let tracer = Tracer::new()
    .with_exporter(Arc::new(OtelExporter) as Arc<dyn SpanExporter>);
```

## 与 BeeAI 的对齐

| BeeAI | Synapse |
|-------|---------|
| `Tracer` | `Tracer` |
| `Span` | `Span` |
| `SpanExporter` | `SpanExporter` |
| `ConsoleExporter` | `StdoutExporter` |
| `Metrics` | `Metrics` |

## Span 层级

ActiveSpan 支持嵌套子 Span：

```rust
let parent = tracer.start_span("parent_op", ComponentType::Agent, "agent1", RunType::Invoke);
let mut child = parent.child("child_op", ComponentType::Tool, "tool1", RunType::Invoke);
// ... child 操作
child.finish();
parent.finish();
```

## 测试

```bash
cargo test --lib observability
```

相关测试覆盖：

- `test_span_creation` / `test_span_error` - Span 基础
- `test_memory_exporter` - 内存导出器
- `test_metrics` - 指标收集
- `test_tracer_handler` - Callback 集成

## 参考

- [BeeAI Framework 文档](https://github.com/i-am-bee/beeai-framework)
- [OpenTelemetry Trace 模型](https://opentelemetry.io/docs/concepts/signals/traces/)