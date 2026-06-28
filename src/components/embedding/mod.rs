use async_trait::async_trait;

pub mod openai;

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("rate limit")]
    RateLimit,
}

#[async_trait]
pub trait Embedding: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> EmbeddingResult<Vec<Vec<f32>>>;

    fn dimensions(&self) -> usize;
}

impl dyn Embedding {
    pub async fn embed_strs(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        let owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        self.embed(owned).await
    }
}
