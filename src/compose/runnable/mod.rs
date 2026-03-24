pub mod stream;
pub mod context;
pub mod error;
pub mod lambda;

pub use stream::{StreamReader, StreamWriter, channel, pipe, from_array, merge, StreamError};
pub use context::{Context, CallbackHandler, RunInfo};
pub use error::{RunnableError, RunnableResult};
pub use lambda::{Lambda, invokable_lambda};

use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Runnable<I, O>: Send + Sync
where
    I: Send + 'static,
    O: Send + 'static,
{
    async fn invoke(&self, ctx: Context, input: I) -> RunnableResult<O>;

    async fn stream(&self, _ctx: Context, _input: I) -> RunnableResult<StreamReader<O>> {
        Err(RunnableError::NotImplemented("stream not implemented".into()))
    }

    async fn collect(&self, _ctx: Context, _input: StreamReader<I>) -> RunnableResult<O> {
        Err(RunnableError::NotImplemented("collect not implemented".into()))
    }

    async fn transform(&self, _ctx: Context, _input: StreamReader<I>) -> RunnableResult<StreamReader<O>> {
        Err(RunnableError::NotImplemented("transform not implemented".into()))
    }

    fn name(&self) -> String {
        "unnamed".to_string()
    }
}

pub fn boxed<I, O>(runnable: impl Runnable<I, O> + 'static) -> Arc<dyn Runnable<I, O>>
where
    I: Send + 'static,
    O: Send + 'static,
{
    Arc::new(runnable)
}
