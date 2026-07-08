//! 结构化输出（Eino 风格）
//!
//! 提供从 LLM 输出提取结构化数据的工具：
//! - `JsonOutputParser<T>` - 类型安全的 JSON 解析
//! - JSON Schema 验证
//! - 自动重试机制
//! - 其他格式解析器（CSV、YAML、Regex）

pub mod json;
pub mod retry;
pub mod csv;
pub mod yaml;
pub mod regex;

pub use json::{JsonOutputParser, JsonSchemaValidator};
pub use retry::{RetryParser, RetryConfig};
pub use csv::CsvOutputParser;
pub use yaml::YamlOutputParser;
pub use regex::RegexOutputParser;

use thiserror::Error;

/// 结构化输出错误
#[derive(Debug, Error)]
pub enum OutputError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("max retries exceeded: {0}")]
    MaxRetries(String),

    #[error("invalid schema: {0}")]
    InvalidSchema(String),

    #[error("io error: {0}")]
    Io(String),
}

pub type OutputResult<T> = Result<T, OutputError>;

/// 输出解析器 trait
pub trait OutputParser<T>: Send + Sync {
    /// 从文本中解析结构化数据
    fn parse(&self, text: &str) -> OutputResult<T>;

    /// 获取 schema（用于传递给 LLM）
    fn schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// 格式化错误信息为可重试的提示
    fn format_error_for_retry(&self, text: &str, error: &OutputError) -> String {
        format!(
            "Your previous response could not be parsed. Error: {}\nPlease provide a valid response.",
            error
        )
    }
}

/// 共享类型
pub type SharedParser<T> = std::sync::Arc<dyn OutputParser<T> + Send + Sync>;
