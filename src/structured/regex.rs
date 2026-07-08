use std::sync::Arc;

use crate::structured::{OutputError, OutputParser, OutputResult};

/// Regex 输出解析器
///
/// 使用正则表达式从 LLM 输出中提取内容。
pub struct RegexOutputParser {
    pattern: Arc<regex::Regex>,
    transform: TransformFn,
}

type TransformFn = Arc<dyn Fn(&str) -> Result<String, OutputError> + Send + Sync>;

impl RegexOutputParser {
    pub fn new(pattern: &str) -> Result<Self, OutputError> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| OutputError::Parse(format!("invalid regex: {e}")))?;
        Ok(Self {
            pattern: Arc::new(regex),
            transform: Arc::new(|s: &str| Ok(s.to_string())),
        })
    }

    pub fn with_transform<F>(pattern: &str, transform: F) -> Result<Self, OutputError>
    where
        F: Fn(&str) -> Result<String, OutputError> + Send + Sync + 'static,
    {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| OutputError::Parse(format!("invalid regex: {e}")))?;
        Ok(Self {
            pattern: Arc::new(regex),
            transform: Arc::new(transform),
        })
    }
}

impl OutputParser<String> for RegexOutputParser {
    fn parse(&self, text: &str) -> OutputResult<String> {
        if let Some(captures) = self.pattern.captures(text) {
            let matched = captures
                .get(0)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            (self.transform)(&matched)
        } else {
            Err(OutputError::Parse(format!(
                "no match for pattern in text: {}",
                text.chars().take(100).collect::<String>()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_basic() {
        let parser = RegexOutputParser::new(r"\d+").unwrap();
        let result: String = parser.parse("There are 42 items").unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_regex_capture_group() {
        let parser = RegexOutputParser::new(r"name=(\w+)").unwrap();
        let result: String = parser.parse("hello name=Alice world").unwrap();
        assert_eq!(result, "name=Alice");
    }

    #[test]
    fn test_regex_no_match() {
        let parser = RegexOutputParser::new(r"\d+").unwrap();
        let result = parser.parse("no digits here");
        assert!(result.is_err());
    }
}