# 结构化输出

`synapse::structured` 模块提供了从 LLM 输出提取结构化数据的工具（Eino 风格）。

## 概述

LLM 输出通常是自然语言文本，但实际应用需要结构化数据。结构化输出模块提供了：

| 解析器 | 说明 | 用途 |
|--------|------|------|
| `JsonOutputParser<T>` | 类型安全的 JSON 解析 | 最常见 |
| `JsonSchemaValidator` | JSON Schema 验证 | 复杂结构 |
| `RetryParser` | 失败自动重试 | 生产环境 |
| `CsvOutputParser<T>` | CSV 解析 | 表格数据 |
| `YamlOutputParser` | YAML 解析 | 配置文件 |
| `RegexOutputParser` | 正则提取 | 简单字段 |

## 统一接口

```rust
pub trait OutputParser<T>: Send + Sync {
    fn parse(&self, text: &str) -> OutputResult<T>;
    fn schema(&self) -> Option<serde_json::Value> { None }
    fn format_error_for_retry(&self, text: &str, error: &OutputError) -> String;
}
```

## 使用示例

### JSON 解析

```rust
use synapse::structured::{JsonOutputParser, OutputParser};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
}

let parser = JsonOutputParser::<Person>::new();
let text = r#"```json
{"name": "Alice", "age": 30}
```"#;
let person: Person = parser.parse(text)?;
```

`JsonOutputParser` 会自动从以下格式提取 JSON：
1. ` ```json ... ``` ` 代码块
2. ` ``` ... ``` ` 代码块
3. 第一个 `{` 到最后一个 `}`
4. 整个文本

### JSON Schema 验证

```rust
use synapse::structured::JsonSchemaValidator;

let schema = serde_json::json!({
    "type": "object",
    "required": ["name"],
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "number"}
    }
});
let validator = JsonSchemaValidator::new(schema);

let value = serde_json::json!({"name": "Alice", "age": 30});
validator.validate(&value)?;
```

### 自动重试

```rust
use synapse::structured::{JsonOutputParser, RetryParser};
use std::sync::Arc;

let parser = Arc::new(JsonOutputParser::<Person>::new());

let retry_fn = Arc::new(|_error_msg: String| {
    // 调用 LLM 重试
    Ok(r#"{"name": "Alice", "age": 30}"#.to_string())
});

let retry_parser = RetryParser::new(parser, retry_fn);
let result = retry_parser.parse_with_retry(initial_text).await?;
```

### CSV 解析

```rust
use synapse::structured::CsvOutputParser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Row {
    name: String,
    age: u32,
}

let parser = CsvOutputParser::<Row>::new();
let text = "name,age\nAlice,30\nBob,25\n";
let rows: Vec<Row> = parser.parse(text)?;
```

### YAML 解析

```rust
use synapse::structured::YamlOutputParser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    name: String,
    version: u32,
}

let parser = YamlOutputParser::new();
let text = r#"
name: myapp
version: 1
"#;
let config: Config = parser.parse(text)?;
```

### Regex 提取

```rust
use synapse::structured::RegexOutputParser;

let parser = RegexOutputParser::new(r"name=(\w+)")?;
let result: String = parser.parse("hello name=Alice world")?;
assert_eq!(result, "name=Alice");
```

## 在 Agent 中使用

```rust
use synapse::structured::{JsonOutputParser, OutputParser};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ActionPlan {
    steps: Vec<String>,
    reasoning: String,
}

let parser = JsonOutputParser::<ActionPlan>::new();
let schema = parser.schema().unwrap();

let prompt = format!(
    "Provide a JSON response matching this schema: {}\nResponse:",
    serde_json::to_string_pretty(&schema).unwrap()
);
let llm_output = call_llm(prompt).await?;
let plan: ActionPlan = parser.parse(&llm_output)?;
```

## 与 Eino 的对应

| Eino | Synapse |
|------|---------|
| `parser.NewJSONResponseParser` | `JsonOutputParser` |
| `parser.NewStructParser` | `JsonOutputParser<T>` |
| `parser.NewRetryParser` | `RetryParser` |
| `parser.NewCSVParser` | `CsvOutputParser` |
| `parser.NewYAMLParser` | `YamlOutputParser` |
| `parser.NewRegExpParser` | `RegexOutputParser` |

## 测试

```bash
cargo test --lib structured
```

15 个测试覆盖所有解析器：
- JSON 解析（4 个）
- Schema 验证（1 个）
- 自动重试（2 个）
- CSV 解析（1 个）
- YAML 解析（2 个）
- Regex 解析（3 个）
- Mod 导出（1 个）

## 性能考虑

| 解析器 | 性能特征 | 内存开销 |
|--------|----------|----------|
| JSON | 快速，O(n) | 中等 |
| CSV | 快速，O(n) | 低 |
| YAML | 中等，O(n²) | 中等 |
| Regex | 取决于 pattern | 极低 |

## 自定义解析器

实现 `OutputParser<T>` trait 即可创建自定义解析器：

```rust
use synapse::structured::{OutputError, OutputParser, OutputResult};

pub struct MyParser;

impl OutputParser<MyType> for MyParser {
    fn parse(&self, text: &str) -> OutputResult<MyType> {
        // 自定义解析逻辑
        // ...
        Ok(my_value)
    }
    
    fn schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({"type": "object"}))
    }
}
```

## 参考

- [Eino 文档](https://www.cloudwego.io/zh/docs/eino/)
- [JSON Output Parser 设计](https://www.cloudwego.io/zh/docs/eino/core_modules/dashscope_output_parser/)