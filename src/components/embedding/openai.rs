use crate::components::embedding::{Embedding, EmbeddingError, EmbeddingResult};
use crate::transport::HttpClient;
use serde_json::json;
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_MODEL: &str = "text-embedding-3-small";

pub struct OpenAIEmbedding {
    http: Arc<dyn HttpClient>,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIEmbedding {
    pub fn new(http: Arc<dyn HttpClient>, api_key: impl Into<String>) -> Self {
        Self {
            http,
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait::async_trait]
impl Embedding for OpenAIEmbedding {
    async fn embed(&self, texts: Vec<String>) -> EmbeddingResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let body = json!({
            "model": self.model,
            "input": texts,
        });
        let url = format!("{}/v1/embeddings", self.base_url);

        let resp: serde_json::Value = self
            .http
            .post_json_with_headers(
                &url,
                &[("Authorization", &format!("Bearer {}", self.api_key))],
                &body,
            )
            .await
            .map_err(|e| EmbeddingError::Embedding(e.to_string()))?;

        if let Some(error_obj) = resp.get("error") {
            let msg = error_obj
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(EmbeddingError::Embedding(format!("OpenAI error: {}", msg)));
        }

        let data = resp["data"]
            .as_array()
            .ok_or_else(|| EmbeddingError::Embedding("missing data array".to_string()))?;

        let mut out = vec![Vec::new(); texts.len()];
        for entry in data {
            let index = entry["index"].as_u64().ok_or_else(|| {
                EmbeddingError::Embedding("missing index in embedding entry".to_string())
            })? as usize;
            let embedding = entry["embedding"]
                .as_array()
                .ok_or_else(|| EmbeddingError::Embedding("missing embedding array".to_string()))?;
            let vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            if index < out.len() {
                out[index] = vec;
            }
        }

        Ok(out)
    }

    fn dimensions(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        }
    }
}
