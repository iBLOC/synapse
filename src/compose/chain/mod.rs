use async_trait::async_trait;

pub type ChainResult<T> = Result<T, ChainError>;

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("chain error: {0}")]
    Chain(String),
    #[error("node error: {0}")]
    Node(String),
}

#[async_trait]
pub trait Chain<I, O>: Send + Sync {
    async fn invoke(&self, input: I) -> ChainResult<O>;
}
