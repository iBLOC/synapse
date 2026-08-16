use serde::de::DeserializeOwned;

use crate::structured::{OutputError, OutputParser, OutputResult};

/// YAML 输出解析器
pub struct YamlOutputParser;

impl YamlOutputParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for YamlOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DeserializeOwned + Send + Sync> OutputParser<T> for YamlOutputParser {
    fn parse(&self, text: &str) -> OutputResult<T> {
        let trimmed = text.trim();

        let yaml_str = if trimmed.starts_with("```yaml") {
            let after = trimmed.strip_prefix("```yaml").unwrap();
            if let Some(end) = after.find("```") {
                after[..end].trim()
            } else {
                after.trim()
            }
        } else if trimmed.starts_with("```") {
            let after = trimmed.strip_prefix("```").unwrap();
            if let Some(end) = after.find("```") {
                let content = after[..end].trim();
                if !content.starts_with('{') && !content.starts_with('[') {
                    content
                } else {
                    return Err(OutputError::Parse(
                        "expected YAML, got JSON-like content".into(),
                    ));
                }
            } else {
                after.trim()
            }
        } else {
            trimmed
        };

        serde_yaml::from_str(yaml_str)
            .map_err(|e| OutputError::Parse(format!("YAML parse error: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: String,
        version: u32,
    }

    #[test]
    fn test_yaml_parse() {
        let yaml = r#"
name: myapp
version: 1
"#;
        let parser = YamlOutputParser::new();
        let config: Config = parser.parse(yaml).unwrap();
        assert_eq!(config.name, "myapp");
        assert_eq!(config.version, 1);
    }

    #[test]
    fn test_yaml_with_markdown() {
        let yaml = "```yaml\nname: test\nversion: 2\n```";
        let parser = YamlOutputParser::new();
        let config: Config = parser.parse(yaml).unwrap();
        assert_eq!(config.name, "test");
    }
}