# 强类型 Lambda 组合

`synapse::compose::runnable::typed_lambda` 模块提供了 Eino 风格的强类型 Lambda 组合能力。

## 概述

Eino（字节跳动开源的 Go LLM 框架）的核心设计哲学之一是**强类型 Lambda 组合**。
Synapse 的 `TypedLambda` 模块对齐了这一设计，允许开发者使用任意输入/输出类型组合 Lambda，
保证编译期类型安全。

## 核心类型

### TypedLambda

```rust
pub struct TypedLambda<I, O> {
    name: String,
    invoke_fn: Arc<dyn Fn(Context, I) -> RunnableFuture<O> + Send + Sync>,
}
```

强类型 Lambda - 支持任意输入/输出类型的可执行单元。

### 组合器

| 组合器 | 说明 |
|--------|------|
| `Chain<L, R>` | 串接两个 Runnable |
| `Parallel<A, B>` | 并行执行两个 Runnable |
| `Router<F, A, B>` | 根据条件路由到不同分支 |

## 使用示例

### 创建强类型 Lambda

```rust
use synapse::compose::runnable::typed_lambda::lambda;
use synapse::compose::runnable::{Context, Runnable};
use std::sync::Arc;

let upper: Arc<TypedLambda<String, String>> = lambda("upper", |_ctx, input| async move {
    Ok(input.to_uppercase())
});
```

### 串接组合

```rust
use synapse::compose::runnable::typed_lambda::{lambda, Chain};

let upper: Arc<TypedLambda<String, String>> = lambda("upper", |_ctx, input| async move {
    Ok(input.to_uppercase())
});

let add_excl: Arc<TypedLambda<String, String>> = lambda("excl", |_ctx, input| async move {
    Ok(format!("{}!", input))
});

let chain = Chain::new(upper, add_excl);
let result = chain.invoke(ctx, "hello".to_string()).await?;
// 输出: "HELLO!"
```

### 并行组合

```rust
use synapse::compose::runnable::typed_lambda::{lambda, Parallel};

let upper: Arc<TypedLambda<String, String>> = lambda("upper", |_ctx, input| async move {
    Ok(input.to_uppercase())
});

let len: Arc<TypedLambda<String, usize>> = lambda("len", |_ctx, input| async move {
    Ok(input.len())
});

let parallel = Parallel::new(upper, len);
let result = parallel.invoke(ctx, "hello".to_string()).await?;
// 输出: ("HELLO", 5)
```

### 条件路由

```rust
use synapse::compose::runnable::typed_lambda::{lambda, Router};

let upper: Arc<TypedLambda<String, String>> = lambda("upper", |_ctx, input| async move {
    Ok(input.to_uppercase())
});

let lower: Arc<TypedLambda<String, String>> = lambda("lower", |_ctx, input| async move {
    Ok(input.to_lowercase())
});

let router = Router::new(
    |_ctx, input| input.as_str().map(|s| s.len() > 3).unwrap_or(false),
    upper,
    lower,
);
```

## 设计原则

### 与 Eino 的对齐

| Eino | Synapse |
|------|---------|
| `Lambda[I, O]` | `TypedLambda<I, O>` |
| `Chain` | `Chain` |
| `Parallel` | `Parallel` |
| `Branch` | `Router` |

### 类型安全

所有 Lambda 的输入输出类型在编译期确定，避免运行时类型错误：

```rust
// 编译通过 - 类型匹配
let ok = upper.invoke(ctx, "hello".to_string()).await?;

// 编译失败 - 类型不匹配
// let err = upper.invoke(ctx, 42i32).await?;
```

## 与普通 Lambda 的区别

普通 `Lambda`（`compose::runnable::lambda`）使用 `serde_json::Value` 作为通用输入/输出，
适合动态场景。`TypedLambda` 提供静态类型检查，适合编译期保证正确性的场景。

## 测试

```bash
cargo test --lib typed_lambda
```

相关测试覆盖：

- `test_typed_lambda` - 基本 Lambda 调用
- `test_parallel` - 并行执行
- `test_lambda_name` - Lambda 命名

## 参考

- [Eino 官方文档](https://www.cloudwego.io/zh/docs/eino/)
- [LangChain Expression Language](https://python.langchain.com/docs/concepts/lcel/)