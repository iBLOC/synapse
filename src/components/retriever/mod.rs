use async_trait::async_trait;

pub use crate::components::document::Document;

pub type RetrieverResult<T> = Result<T, RetrieverError>;

#[derive(Debug, thiserror::Error)]
pub enum RetrieverError {
    #[error("retrieve error: {0}")]
    Retrieve(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(
        &self,
        query: &str,
        options: RetrieveOptions,
    ) -> RetrieverResult<Vec<Document>>;
}

pub struct RetrieveOptions {
    pub top_k: Option<usize>,
    pub threshold: Option<f32>,
}

impl Default for RetrieveOptions {
    fn default() -> Self {
        Self {
            top_k: Some(4),
            threshold: None,
        }
    }
}
