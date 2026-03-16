use async_trait::async_trait;
use std::collections::HashMap;

pub type DocumentResult<T> = Result<T, DocumentError>;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("io error: {0}")]
    Io(String),
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[async_trait]
pub trait DocumentParser: Send + Sync {
    fn supported_mime_types(&self) -> Vec<&'static str>;
    
    async fn parse(&self, content: &[u8], mime_type: &str) -> DocumentResult<ParsedDocument>;
    
    fn can_parse(&self, mime_type: &str) -> bool {
        self.supported_mime_types().contains(&mime_type)
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Document {
    pub fn new(id: impl Into<String>, content: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            mime_type: mime_type.into(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

pub struct TextParser;

impl TextParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentParser for TextParser {
    fn supported_mime_types(&self) -> Vec<&'static str> {
        vec!["text/plain", "text/markdown", "text/html"]
    }
    
    async fn parse(&self, content: &[u8], mime_type: &str) -> DocumentResult<ParsedDocument> {
        if !self.can_parse(mime_type) {
            return Err(DocumentError::UnsupportedFormat(mime_type.to_string()));
        }
        
        let content = String::from_utf8(content.to_vec())
            .map_err(|e| DocumentError::Parse(e.to_string()))?;
        
        Ok(ParsedDocument {
            content,
            metadata: HashMap::new(),
        })
    }
}

impl Default for TextParser {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JsonParser;

impl JsonParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DocumentParser for JsonParser {
    fn supported_mime_types(&self) -> Vec<&'static str> {
        vec!["application/json"]
    }
    
    async fn parse(&self, content: &[u8], mime_type: &str) -> DocumentResult<ParsedDocument> {
        if !self.can_parse(mime_type) {
            return Err(DocumentError::UnsupportedFormat(mime_type.to_string()));
        }
        
        let content = String::from_utf8(content.to_vec())
            .map_err(|e| DocumentError::Parse(e.to_string()))?;
        
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| DocumentError::Parse(e.to_string()))?;
        
        let content = serde_json::to_string(&json)
            .unwrap_or(content);
        
        Ok(ParsedDocument {
            content,
            metadata: HashMap::new(),
        })
    }
}

impl Default for JsonParser {
    fn default() -> Self {
        Self::new()
    }
}
