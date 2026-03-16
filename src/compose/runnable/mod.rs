use async_trait::async_trait;
use std::sync::Arc;

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

#[async_trait]
pub trait Runnable<I, O>: Send + Sync {
    async fn invoke(&self, input: I) -> RunnableResult<O>;
    
    fn name(&self) -> String {
        "unnamed".to_string()
    }
}

pub struct RunnableMap<I, O> {
    runnable: Box<dyn Runnable<I, O>>,
    name: String,
}

impl<I, O> RunnableMap<I, O> {
    pub fn new(runnable: Box<dyn Runnable<I, O>>, name: impl Into<String>) -> Self {
        Self {
            runnable,
            name: name.into(),
        }
    }
}

#[async_trait]
impl<I: Send, O: Send> Runnable<I, O> for RunnableMap<I, O> {
    async fn invoke(&self, input: I) -> RunnableResult<O> {
        self.runnable.invoke(input).await
    }
    
    fn name(&self) -> String {
        self.name.clone()
    }
}

pub fn boxed<I, O>(runnable: impl Runnable<I, O> + 'static) -> Box<dyn Runnable<I, O>> {
    Box::new(runnable)
}

pub fn boxed_arc<I, O>(runnable: impl Runnable<I, O> + 'static) -> Arc<dyn Runnable<I, O>> {
    Arc::new(runnable)
}
