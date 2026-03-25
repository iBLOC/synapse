pub type RunnableResult<T> = Result<T, RunnableError>;

#[derive(Debug, thiserror::Error)]
pub enum RunnableError {
    #[error("invoke error: {0}")]
    Invoke(String),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("batch error: {0}")]
    Batch(String),
}
