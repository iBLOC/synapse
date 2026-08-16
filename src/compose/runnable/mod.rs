pub mod context;
pub mod error;
pub mod lambda;
pub mod pipe;
pub mod stream;
pub mod typed_lambda;

pub use context::{Context, Handler, RunInfo};
pub use error::{RunnableError, RunnableResult};
pub use lambda::{invokable_lambda, Lambda};
pub use pipe::{pipe, prompt_template, string_parser, json_parser, JsonOutputParser, Pipe, PromptTemplate, Sequence, StringOutputParser};
pub use stream::{channel, from_array, merge, pipe as stream_pipe, StreamError, StreamReader, StreamWriter};
pub use typed_lambda::{lambda as typed_lambda_fn, Parallel, Router, TypedLambda};

use std::pin::Pin;
use std::sync::Arc;

pub type RunnableFuture<O> =
    Pin<Box<dyn std::future::Future<Output = RunnableResult<O>> + Send + 'static>>;

pub trait Runnable<I, O>: Send + Sync
where
    I: Send + 'static,
    O: Send + 'static,
{
    fn invoke(&self, _ctx: Context, _input: I) -> RunnableFuture<O> {
        Box::pin(async move {
            Err(RunnableError::NotImplemented(
                "invoke not implemented".into(),
            ))
        })
    }

    fn stream(&self, _ctx: Context, _input: I) -> RunnableFuture<StreamReader<O>> {
        Box::pin(async move {
            Err(RunnableError::NotImplemented(
                "stream not implemented".into(),
            ))
        })
    }

    fn collect(&self, _ctx: Context, _input: StreamReader<I>) -> RunnableFuture<O> {
        Box::pin(async move {
            Err(RunnableError::NotImplemented(
                "collect not implemented".into(),
            ))
        })
    }

    fn transform(&self, _ctx: Context, _input: StreamReader<I>) -> RunnableFuture<StreamReader<O>> {
        Box::pin(async move {
            Err(RunnableError::NotImplemented(
                "transform not implemented".into(),
            ))
        })
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
