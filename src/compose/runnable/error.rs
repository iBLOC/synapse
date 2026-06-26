use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnableError {
    #[error("invoke error: {0}")]
    Invoke(String),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("collect error: {0}")]
    Collect(String),
    #[error("transform error: {0}")]
    Transform(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("graph already compiled")]
    GraphCompiled,
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type RunnableResult<T> = Result<T, RunnableError>;
