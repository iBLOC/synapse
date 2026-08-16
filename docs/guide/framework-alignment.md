# 框架对齐：Eino / BeeAI / LangChain

本文档说明 Synapse 的三个新模块如何对齐业界主流框架的设计。

## 概述

| 改进 | 对标框架 | 设计理念 |
|------|----------|----------|
| `typed_lambda` | Eino (字节跳动) | 强类型 Lambda 组合 |
| `pipe` | LangChain LCEL | 链式表达式语法 |
| `observability` | BeeAI Framework | 生产级可观测性 |

## 1. 强类型 Lambda (typed_lambda) — Eino 风格

### Eino 的设计

Eino（cloudwego/eino）使用 Go 的强类型系统实现 Lambda 组合：

```go
// Eino 示例
chain := compose.NewChain[map[string]any, map[string]any]()
chain.AppendLambda(compose.InvokableLambda(func(ctx context.Context, in map[string]any) (map[string]any, error) {
    return map[string]any{"result": in["input"]}, nil
}))
```

### Synapse 的实现

```rust
// Synapse 示例
let lambda: Arc<TypedLambda<String, String>> = lambda("upper", |_ctx, input| async move {
    Ok(input.to_uppercase())
});
```

### 设计对比

| 特性 | Eino | Synapse |
|------|------|---------|
| 类型安全 | ✅ Go 静态类型 | ✅ Rust 静态类型 |
| 串接组合 | `Chain` | `Chain` |
| 并行组合 | `Parallel` | `Parallel` |
| 条件路由 | `Branch` | `Router` |
| 异步执行 | ✅ | ✅ |
| 流式支持 | ✅ | ✅（继承自 Runnable） |

## 2. 链式表达式 (pipe) — LCEL 风格

### LCEL 的设计

LangChain Expression Language 用 `|` 操作符组合组件：

```python
# LCEL 示例
chain = prompt | llm | output_parser
result = chain.invoke({"input": "hello"})
```

### Synapse 的实现

由于 Rust 不允许重载 `|` 操作符作用于任意类型，Synapse 使用辅助函数：

```rust
// Synapse 示例
let chain = pipe(
    Arc::new(prompt_template("Hello {input}")),
    pipe(
        Arc::new(llm_call),
        Arc::new(string_parser()),
    ),
);
```

### 设计对比

| 特性 | LCEL | Synapse |
|------|------|---------|
| 链式组合 | `\|` 操作符 | `pipe()` 函数 |
| 提示模板 | ✅ | ✅ |
| 输出解析 | ✅ | ✅ |
| 流式 | ✅ | ✅ |
| 异步 | ✅ | ✅ |
| 编译期类型 | ❌ | ✅ |

### 为什么不直接用 `|` 操作符

Rust 的 `BitOr` trait 实现有严格的孤儿规则限制：

```rust
// 不能直接这样写：
let chain = step1 | step2 | step3;
```

需要为每个具体类型实现 `BitOr`，违反孤儿规则。Synapse 选择 `pipe()` 函数作为替代方案。

## 3. 可观测性 (observability) — BeeAI 风格

### BeeAI 的设计

BeeAI Framework 强调生产部署的可观测性：

```python
# BeeAI 示例
from beeai_framework.observability import Tracer, Span

tracer = Tracer()
with tracer.span("my_operation") as span:
    span.set_input({"query": "hello"})
    # ... 操作
    span.set_output({"response": "world"})
```

### Synapse 的实现

```rust
// Synapse 示例
let tracer = Tracer::new();
let mut span = tracer.start_span("my_op", ComponentType::ChatModel, "gpt-4", RunType::Invoke);
span.set_input(json!({"query": "hello"}));
// ... 操作
span.set_output(json!({"response": "world"}));
let _ = span.finish();
```

### 设计对比

| 特性 | BeeAI | Synapse |
|------|-------|---------|
| Span 追踪 | ✅ | ✅ |
| 层级 Span | ✅ | ✅ |
| 可插拔导出器 | ✅ | ✅ |
| 指标收集 | ✅ | ✅ |
| OpenTelemetry 集成 | ✅ | 🔜 通过自定义 Exporter |
| 编译期保证 | ❌ | ✅ |

## 总结

Synapse 通过借鉴业界主流框架的优秀设计，同时保留 Rust 的强类型和零成本抽象优势：

1. **类型安全** — 编译期保证，减少运行时错误
2. **零成本抽象** — 与手写代码性能相当
3. **异步原生** — 充分利用 Tokio 运行时
4. **可组合性** — 模块化设计，便于扩展

## 参考

- [Eino](https://github.com/cloudwego/eino) - 字节跳动开源的 Go LLM 框架
- [BeeAI Framework](https://github.com/i-am-bee/beeai-framework) - IBM 开源的企业级 Agent 框架
- [LangChain](https://github.com/langchain-ai/langchain) - 最早的 Python LLM 框架