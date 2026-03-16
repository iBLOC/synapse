use async_trait::async_trait;

#[async_trait]
pub trait Node<I, O>: Send + Sync {
    async fn invoke(&self, input: I) -> Result<O, NodeError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("node error: {0}")]
    Node(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
