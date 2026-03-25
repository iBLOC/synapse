use async_trait::async_trait;
use std::marker::PhantomData;
use super::error::RunnableResult;

/// Lambda is a simple function wrapper that implements Runnable
pub struct Lambda<I, O, F> {
    func: F,
    name: String,
    _phantom: PhantomData<fn(I) -> O>,
}

impl<I, O, F> Lambda<I, O, F>
where
    F: Fn(I) -> RunnableResult<O> + Send + Sync + 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            name: "lambda".to_string(),
            _phantom: PhantomData,
        }
    }
    
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

#[async_trait]
impl<I: Send, O: Send, F> super::Runnable<I, O> for Lambda<I, O, F>
where
    F: Fn(I) -> RunnableResult<O> + Send + Sync + 'static,
{
    async fn invoke(&self, input: I) -> RunnableResult<O> {
        (self.func)(input)
    }
    
    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Async lambda for async functions
pub struct AsyncLambda<I, O, F> {
    func: F,
    name: String,
    _phantom: PhantomData<fn(I) -> O>,
}

impl<I, O, F> AsyncLambda<I, O, F>
where
    F: Fn(I) -> RunnableResult<O> + Send + Sync + 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            name: "async_lambda".to_string(),
            _phantom: PhantomData,
        }
    }
    
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

#[async_trait]
impl<I: Send, O: Send, F> super::Runnable<I, O> for AsyncLambda<I, O, F>
where
    F: Fn(I) -> RunnableResult<O> + Send + Sync + 'static,
{
    async fn invoke(&self, input: I) -> RunnableResult<O> {
        (self.func)(input)
    }
    
    fn name(&self) -> String {
        self.name.clone()
    }
}
