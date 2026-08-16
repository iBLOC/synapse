# OpenTelemetry 集成评估

本文档评估将 Synapse 的 `observability` 模块与 OpenTelemetry (OTel) 集成的方案。

## 概述

### 当前 observability 模块

`synapse::callbacks::observability` 已实现：
- `Span` / `SpanStatus` — Span 状态追踪
- `Tracer` / `ActiveSpan` — Span 创建和管理
- `SpanExporter` trait — 可插拔导出器接口
- `StdoutExporter` / `MemoryExporter` — 内置导出器
- `TracerHandler` — 集成到现有 Callback 系统
- `Metrics` / `MetricsHandler` — 指标收集

### OpenTelemetry 概览

OpenTelemetry (OTel) 是 CNCF 下的可观测性标准：
- **统一 API** — 跨语言、跨厂商的可观测性接口
- **Trace + Metrics + Logs** — 三大支柱
- **OTLP 协议** — 标准的可观测性数据传输协议
- **W3C Trace Context** — 标准的 trace 上下文传播
- **多后端支持** — Jaeger、Zipkin、Tempo、Datadog 等

## OpenTelemetry Rust 生态

### 核心 crate（最新版本 ~0.27+）

| Crate | 用途 | 是否必需 |
|-------|------|----------|
| `opentelemetry` | 核心 API（traits） | ✅ 必需 |
| `opentelemetry_sdk` | SDK 实现 | ✅ 必需 |
| `opentelemetry-otlp` | OTLP exporter | ⚠️ 推荐 |
| `opentelemetry-stdout` | stdout exporter（调试） | ❌ 可选 |
| `opentelemetry-semantic-conventions` | 语义约定常量 | ⚠️ 推荐 |
| `opentelemetry-proto` | ProtoBuf 定义 | ❌ 仅 OTLP gRPC |

### 版本对齐

```toml
[dependencies]
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic", "trace"] }
opentelemetry-semantic-conventions = "0.27"
```

### 关键特性

1. **跨厂商兼容** — 通过 OTLP 协议支持所有主流后端
2. **W3C Trace Context** — 自动注入和提取 trace context
3. **采样支持** — 多种采样策略（AlwaysOn, ParentBased, TraceIdRatioBased）
4. **批处理** — 自动批量发送，降低网络开销
5. **零成本抽象** — 未启用时几乎无性能影响

## 集成架构设计

### 方案一：SpanExporter 适配器（推荐）

将 OTel `Tracer` 包装为 Synapse `SpanExporter`，复用现有 Span 数据结构。

```rust
use opentelemetry::trace::{Tracer as OtelTracer, Span as OtelSpan};
use opentelemetry::Context as OtelContext;
use opentelemetry_sdk::trace::TracerProvider;

pub struct OtelSpanExporter {
    tracer: OtelTracer,
    service_name: String,
}

impl OtelSpanExporter {
    pub fn new(provider: TracerProvider, service_name: impl Into<String>) -> Self {
        Self {
            tracer: provider.tracer(service_name),
            service_name: service_name.into(),
        }
    }
}

impl SpanExporter for OtelSpanExporter {
    fn export(&self, span: &Span) {
        // 转换 Synapse Span → OTel Span
        let mut otel_span = self.tracer
            .span_builder(span.name.clone())
            .with_kind(SpanKind::Internal)
            .start_with_context(
                self.tracer.span_builder(span.name.clone()),
                &OtelContext::new(),
            );
        
        // 设置属性
        otel_span.set_attribute("synapse.span.id", span.id.clone());
        otel_span.set_attribute("synapse.trace.id", span.trace_id.clone());
        otel_span.set_attribute("synapse.component", format!("{:?}", span.component));
        
        // 设置输入输出
        if let Some(input) = &span.input {
            otel_span.set_attribute("synapse.input", input.to_string());
        }
        
        // 设置状态
        match span.status {
            SpanStatus::Ok => {},
            SpanStatus::Error => otel_span.set_status(Status::error(span.error.clone().unwrap_or_default())),
            SpanStatus::Running => {},
        }
        
        otel_span.end();
    }
}
```

**优点**：
- 最小侵入性，复用现有 Span 数据结构
- 不破坏现有 API
- 渐进式迁移

**缺点**：
- 字段映射可能不完整
- 需要手动维护 Synapse Span → OTel Span 的映射

### 方案二：直接使用 OTel API（替代方案）

直接使用 OTel API 重写 observability 模块，放弃自定义 Span 数据结构。

```rust
use opentelemetry::trace::{Tracer, SpanBuilder, SpanKind};
use opentelemetry::global;

pub struct OtelTracer {
    tracer: opentelemetry::global::BoxedTracer,
}

impl OtelTracer {
    pub fn new() -> Self {
        Self {
            tracer: global::tracer("synapse"),
        }
    }
    
    pub fn start_span(&self, name: &str) -> OtelActiveSpan {
        let span = self.tracer
            .span_builder(name.to_string())
            .with_kind(SpanKind::Internal)
            .start(&self.tracer);
        OtelActiveSpan { span }
    }
}

pub struct OtelActiveSpan {
    span: opentelemetry::global::BoxedSpan,
}

impl OtelActiveSpan {
    pub fn set_attribute(&mut self, key: &str, value: impl Into<AttributeValue>) {
        self.span.set_attribute(key, value);
    }
    
    pub fn set_input(&mut self, input: serde_json::Value) {
        self.span.set_attribute("synapse.input", input.to_string());
    }
    
    pub fn finish(self) {
        self.span.end();
    }
}
```

**优点**：
- 与 OTel 生态完全对齐
- 享受 OTel 全部特性（W3C Context、采样等）
- 标准化的数据模型

**缺点**：
- 重写整个 observability 模块
- 破坏现有 API（向后不兼容）
- 引入 OTel 依赖（编译时间、依赖大小）

### 方案三：双轨模式（推荐用于生产）

同时保留自定义 Span 和 OTel 支持，用户按需启用。

```rust
pub enum TracerImpl {
    Builtin(Tracer),                  // 内置实现（无 OTel 依赖）
    Otel(OtelTracerWrapper),          // OTel 实现（启用 OTel feature）
}

pub fn create_tracer(config: &TracerConfig) -> TracerImpl {
    if config.opentelemetry_enabled {
        TracerImpl::Otel(OtelTracerWrapper::new(config))
    } else {
        TracerImpl::Builtin(Tracer::new())
    }
}
```

通过 Cargo feature 控制：
```toml
[features]
default = []
opentelemetry = ["dep:opentelemetry", "dep:opentelemetry_sdk", "dep:opentelemetry-otlp"]
```

**优点**：
- 默认无 OTel 依赖（轻量）
- 用户按需启用
- 既有 API 完全兼容

**缺点**：
- 实现复杂度增加
- 需要维护双套实现

## 推荐方案

**推荐方案三：双轨模式 + SpanExporter 适配器**

理由：
1. **零默认开销** — 不启用 OTel feature 时无额外依赖
2. **渐进迁移** — 用户可按需启用，逐步迁移
3. **完整兼容** — SpanExporter 适配器模式复用现有代码
4. **生态标准** — 启用 OTel 后享受标准化的可观测性

## 实施步骤

### 第一阶段：依赖与 feature 隔离

```toml
[features]
default = []
opentelemetry = [
    "dep:opentelemetry",
    "dep:opentelemetry_sdk",
    "dep:opentelemetry-otlp",
    "dep:opentelemetry-semantic-conventions",
]

[dependencies]
opentelemetry = { version = "0.27", optional = true }
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"], optional = true }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic"], optional = true }
opentelemetry-semantic-conventions = { version = "0.27", optional = true }
```

### 第二阶段：实现 SpanExporter 适配器

创建 `src/callbacks/otel_exporter.rs`：

```rust
#[cfg(feature = "opentelemetry")]
pub struct OtelSpanExporter { /* ... */ }

#[cfg(feature = "opentelemetry")]
impl SpanExporter for OtelSpanExporter {
    fn export(&self, span: &Span) {
        // 转换并发送
    }
}
```

### 第三阶段：TracerProvider 辅助函数

```rust
#[cfg(feature = "opentelemetry")]
pub fn init_otel_tracer(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<OtelTracerProvider, Error> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()?;
    
    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            service_name.to_string(),
        )]))
        .build();
    
    global::set_tracer_provider(provider.clone());
    Ok(provider)
}
```

### 第四阶段：自动注入 W3C Trace Context

```rust
#[cfg(feature = "opentelemetry")]
pub fn inject_trace_context(headers: &mut HashMap<String, String>) {
    use opentelemetry::global;
    let cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HashMap::<String, String>::new())
    });
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut HeaderInjector(headers))
    });
}
```

## 性能评估

### 默认（无 OTel）性能

- 无额外依赖
- 零运行时开销
- 现有 Span/Tracer 性能保持不变

### 启用 OTel 性能影响

| 操作 | 影响 |
|------|------|
| Span 创建 | +1~2 µs（哈希+分配） |
| 属性设置 | +100~500 ns/属性 |
| 批处理导出 | 异步，不阻塞调用 |
| 内存开销 | +200~500 bytes/span |

总体而言，OTel 的开销在生产可接受范围内（<5% CPU）。

## 与现有 observability 模块的关系

```
现有 observability (无 OTel)
├── Span (结构体)
├── Tracer
├── ActiveSpan
├── SpanExporter trait
├── StdoutExporter
├── MemoryExporter
└── TracerHandler

新增 otel_exporter (feature-gated)
├── OtelSpanExporter        # 实现 SpanExporter
├── OtelTracerProvider      # OTel SDK 包装
├── W3C Context 注入/提取
└── OTLP 配置辅助
```

**保持 SpanExporter trait 作为统一抽象**，OTel 只是另一个实现。

## 测试策略

### 单元测试

```rust
#[cfg(test)]
#[cfg(feature = "opentelemetry")]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_to_otel_conversion() {
        let span = create_test_span();
        let exporter = OtelSpanExporter::new(...);
        exporter.export(&span);
        // 验证 OTel span 已创建
    }
    
    #[test]
    fn test_otel_exporter_with_error() {
        let span = create_test_span_with_error();
        let exporter = OtelSpanExporter::new(...);
        exporter.export(&span);
        // 验证 OTel span 状态为 Error
    }
}
```

### 集成测试

使用 OTel 测试 SDK（opentelemetry_sdk::testing）验证：
- Span 转换正确性
- 属性映射正确性
- W3C Context 传播

## 后端兼容性

通过 OTLP 协议支持的常见后端：

| 后端 | OTLP 支持 | 推荐协议 |
|------|-----------|----------|
| Jaeger | ✅ | gRPC |
| Tempo | ✅ | gRPC |
| Datadog | ✅ | gRPC |
| New Relic | ✅ | gRPC |
| Honeycomb | ✅ | gRPC |
| AWS X-Ray | ✅ | gRPC |
| Google Cloud Trace | ✅ | gRPC |

## 迁移路径

### Phase 1：非破坏性（1-2 周）

- 添加 OTel feature 和可选依赖
- 实现 OtelSpanExporter
- 文档和示例
- 保持现有 API 不变

### Phase 2：文档和工具（1 周）

- 添加集成指南
- 添加 docker-compose 示例（Jaeger/Tempo）
- 添加性能基准

### Phase 3：增强（按需）

- 实现自动注入 Context
- 实现 Metrics 导出器
- 实现 Logs 集成

## 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| OTel API 变更 | 中 | 中 | 使用稳定版本 (0.27+) |
| 编译时间增加 | 高 | 低 | feature 隔离 |
| 运行时性能 | 低 | 中 | 批处理+异步 |
| W3C Context 兼容 | 低 | 高 | 完整测试 |

## 决策建议

### 立即行动 ✅

1. 在 Cargo.toml 添加 OTel feature 隔离
2. 实现 OtelSpanExporter（基础版本）
3. 编写集成文档

### 后续跟进 🔜

1. 实现 W3C Context 注入/提取
2. 添加 Metrics Exporter
3. 添加 Logs 集成
4. 性能基准测试

### 不建议 ❌

1. 用 OTel API 完全重写 observability 模块（破坏 API）
2. 在默认 feature 中包含 OTel（增加依赖）
3. 强制要求用户使用 OTel（破坏灵活性）

## 参考

- [OpenTelemetry Rust 文档](https://docs.rs/opentelemetry/)
- [OpenTelemetry Specification](https://opentelemetry.io/docs/specs/otel/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [OTLP Protocol](https://opentelemetry.io/docs/specs/otlp/)