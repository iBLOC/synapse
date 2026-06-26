use async_trait::async_trait;

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
