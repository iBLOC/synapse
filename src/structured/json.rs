use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::structured::{OutputError, OutputParser, OutputResult};

/// JSON 输出解析器
///
/// 从 LLM 输出中提取 JSON 并解析为指定类型。
/// 支持自动从 markdown code block 中提取 JSON。
pub struct JsonOutputParser<T> {
    schema: Option<serde_json::Value>,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync> JsonOutputParser<T> {
    pub fn new() -> Self {
        Self {
            schema: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_schema(schema: serde_json::Value) -> Self {
        Self {
            schema: Some(schema),
            _phantom: PhantomData,
        }
    }

    /// 从 LLM 输出中提取 JSON
    ///
    /// 尝试以下策略：
    /// 1. 查找 ```json ... ``` 代码块
    /// 2. 查找 ``` ... ``` 代码块
    /// 3. 查找第一个 { 到最后一个 } 的内容
    /// 4. 整个文本作为 JSON
    pub fn extract_json(text: &str) -> Option<&str> {
        if let Some(start) = text.find("```json") {
            let after = &text[start + 7..];
            if let Some(end) = after.find("```") {
                return Some(after[..end].trim());
            }
        }

        if let Some(start) = text.find("```") {
            let after = &text[start + 3..];
            if let Some(end) = after.find("```") {
                let content = after[..end].trim();
                if content.starts_with('{') || content.starts_with('[') {
                    return Some(content);
                }
            }
        }

        if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
            if start < end {
                return Some(&text[start..=end]);
            }
        }

        if let (Some(start), Some(end)) = (text.find('['), text.rfind(']')) {
            if start < end {
                return Some(&text[start..=end]);
            }
        }

        let trimmed = text.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            return Some(trimmed);
        }

        None
    }
}

impl<T: DeserializeOwned + Send + Sync> Default for JsonOutputParser<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser<T> for JsonOutputParser<T> {
    fn parse(&self, text: &str) -> OutputResult<T> {
        let json_str = Self::extract_json(text).ok_or_else(|| {
            OutputError::Parse(format!(
                "no JSON found in text: {}",
                text.chars().take(100).collect::<String>()
            ))
        })?;

        serde_json::from_str::<T>(json_str).map_err(|e| {
            OutputError::Parse(format!(
                "failed to deserialize into {}: {}",
                std::any::type_name::<T>(),
                e
            ))
        })
    }

    fn schema(&self) -> Option<serde_json::Value> {
        self.schema.clone()
    }
}

/// JSON Schema 验证器
///
/// 使用简单的 JSON Schema 验证（不依赖外部库）。
/// 支持基础类型验证：type, required, properties。
pub struct JsonSchemaValidator {
    schema: serde_json::Value,
}

impl JsonSchemaValidator {
    pub fn new(schema: serde_json::Value) -> Self {
        Self { schema }
    }

    pub fn from_type<T: serde::Serialize>(_phantom: std::marker::PhantomData<T>) -> Option<Self> {
        let value = serde_json::json!({
            "type": "object",
            "description": std::any::type_name::<T>(),
        });
        Some(Self::new(value))
    }

    pub fn validate(&self, value: &serde_json::Value) -> Result<(), String> {
        if let Some(expected_type) = self.schema.get("type").and_then(|v| v.as_str()) {
            let actual_type = match value {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
            };
            if expected_type != actual_type {
                return Err(format!(
                    "expected type '{}', got '{}'",
                    expected_type, actual_type
                ));
            }
        }

        if let Some(required) = self.schema.get("required").and_then(|v| v.as_array()) {
            let obj = value.as_object().ok_or("value is not an object")?;
            for field in required {
                let field_name = field.as_str().ok_or("invalid required field name")?;
                if !obj.contains_key(field_name) {
                    return Err(format!("missing required field '{}'", field_name));
                }
            }
        }

        if let Some(properties) = self.schema.get("properties").and_then(|v| v.as_object()) {
            let obj = value.as_object().ok_or("value is not an object")?;
            for (field_name, field_schema) in properties {
                if let Some(field_value) = obj.get(field_name) {
                    let sub_validator = Self::new(field_schema.clone());
                    sub_validator.validate(field_value)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Person {
        name: String,
        age: u32,
    }

    #[test]
    fn test_extract_json_plain() {
        let text = r#"{"name": "Alice", "age": 30}"#;
        let extracted = JsonOutputParser::<Person>::extract_json(text).unwrap();
        assert_eq!(extracted, r#"{"name": "Alice", "age": 30}"#);
    }

    #[test]
    fn test_extract_json_markdown() {
        let text = r#"Here is the JSON:
```json
{"name": "Alice", "age": 30}
```
"#;
        let extracted = JsonOutputParser::<Person>::extract_json(text).unwrap();
        assert!(extracted.contains("Alice"));
    }

    #[test]
    fn test_extract_json_with_prefix() {
        let text = r#"The result is: {"name": "Bob", "age": 25} as you can see."#;
        let extracted = JsonOutputParser::<Person>::extract_json(text).unwrap();
        assert!(extracted.contains("Bob"));
    }

    #[test]
    fn test_extract_json_array() {
        let text = r#"[{"a": 1}, {"a": 2}]"#;
        let extracted = JsonOutputParser::<Vec<Person>>::extract_json(text).unwrap();
        assert!(extracted.contains("1"));
    }

    #[test]
    fn test_parse_success() {
        let parser = JsonOutputParser::<Person>::new();
        let text = r#"{"name": "Alice", "age": 30}"#;
        let person: Person = parser.parse(text).unwrap();
        assert_eq!(person.name, "Alice");
        assert_eq!(person.age, 30);
    }

    #[test]
    fn test_parse_failure() {
        let parser = JsonOutputParser::<Person>::new();
        let result = parser.parse("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_validation() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            }
        });
        let validator = JsonSchemaValidator::new(schema);

        let valid = serde_json::json!({"name": "Alice", "age": 30});
        assert!(validator.validate(&valid).is_ok());

        let invalid = serde_json::json!({"age": 30});
        assert!(validator.validate(&invalid).is_err());
    }
}