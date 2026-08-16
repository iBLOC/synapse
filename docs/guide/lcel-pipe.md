# LCEL 风格链式表达式

`synapse::compose::runnable::pipe` 模块提供了类似 LangChain Expression Language (LCEL) 的链式表达式组合能力。

## 概述

LangChain 的 LCEL（LangChain Expression Language）使用 `|` 操作符来组合提示模板、LLM 和输出解析器：
```python
chain = prompt | llm | output_parser
```

Synapse 的 `pipe` 模块提供类似的能力，但使用 Rust 的类型系统保证编译期安全。

## 核心 API

### Pipe / JsonPipe

```rust
pub struct Pipe<L, R>(Arc<L>, Arc<R>);
pub type JsonPipe<L, R> = Pipe<L, R>;
```

类型安全的链式管道。使用 Arc 存储，可共享。

### 辅助函数

```rust
pub fn pipe<L, R>(left: Arc<L>, right: Arc<R>) -> Pipe<L, R>
```

创建管道。左右两侧必须实现 `Runnable<serde_json::Value, serde_json::Value>`。

### 内置组件

| 类型 | 说明 |
|------|------|
| `PromptTemplate` | 提示模板，支持 `{input}` 占位符 |
| `StringOutputParser` | 字符串输出解析器 |
| `JsonOutputParser` | JSON 输出解析器 |
| `Sequence` | 多步骤序列组合 |

## 使用示例

### 基本管道

```rust
use synapse::compose::runnable::pipe::*;
use synapse::compose::runnable::Lambda;

let upper = Lambda::invokable(
    "upper",
    |_ctx, input| async move {
        let s = input.as_str().unwrap_or("").to_uppercase();
        Ok(serde_json::Value::String(s))
    },
);

let lower = Lambda::invokable(
    "lower",
    |_ctx, input| async move {
        let s = input.as_str().unwrap_or("").to_lowercase();
        Ok(serde_json::Value::String(s))
    },
);

let p = pipe(Arc::new(upper), Arc::new(lower));
let result = p.invoke(ctx, serde_json::json!("Hello")).await?;
// 输出: "hello" (先大写再小写)
```

### Prompt 模板

```rust
let template = prompt_template("Hello {input}!");
let result = template.invoke(ctx, "World".to_string()).await?;
// 输出: "Hello World!"
```

### 输出解析器

```rust
// 字符串解析器
let parser = string_parser();
let result = parser.invoke(ctx, serde_json::json!("hello")).await?;
// 输出: "hello"

// JSON 解析器
let parser = json_parser();
let result = parser
    .invoke(ctx, serde_json::json!("{\"key\": \"value\"}"))
    .await?;
// 输出: {"key": "value"}
```

### 多步骤序列

```rust
let seq = Sequence::new()
    .then(step1)
    .then(step2)
    .then(step3);

let result = seq.invoke(ctx, input).await?;
```

## 设计选择

### 为什么使用 Arc 存储？

为了支持 `Runnable` 不可 Clone 的情况（如含闭包的 Lambda），Pipe 内部使用 `Arc<L>` 和 `Arc<R>`。
这避免了强制要求所有 Runnable 实现 Clone。

### 为什么使用 JSON Value？

为了统一输入/输出类型并与现有的 `Runnable<serde_json::Value, serde_json::Value>` 兼容。
未来可以考虑实现更细粒度的泛型推导。

## 与 LCEL 的对比

| LCEL (Python) | Synapse |
|---------------|---------|
| `prompt \| llm \| parser` | `pipe(prompt, pipe(llm, parser))` |
| 动态类型 | 编译期类型安全 |
| 运行时错误检测 | 编译期错误检测 |

## 测试

```bash
cargo test --lib pipe
```

相关测试覆盖：

- `test_pipe_json` - 基本管道
- `test_prompt_template` - 模板渲染
- `test_string_output_parser` / `test_json_output_parser` - 解析器
- `test_sequence` - 多步骤序列

## 参考

- [LangChain LCEL](https://python.langchain.com/docs/concepts/lcel/)
- [LangChain Runnable 接口](https://python.langchain.com/docs/concepts/runnables/)