use super::{Context, Runnable, RunnableError, RunnableFuture, RunnableResult};
use std::pin::Pin;
use std::sync::Arc;

pub struct Lambda {
    name: String,
    invoke_fn: Option<
        Box<dyn Send + Sync + Fn(Context, serde_json::Value) -> RunnableFuture<serde_json::Value>>,
    >,
}

impl Lambda {
    pub fn invokable<F, Fut>(name: &str, f: F) -> Self
    where
        F: Fn(Context, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RunnableResult<serde_json::Value>> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            invoke_fn: Some(Box::new(move |ctx, input| Box::pin(f(ctx, input)))),
        }
    }
}

impl std::fmt::Debug for Lambda {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lambda").field("name", &self.name).finish()
    }
}

impl Runnable<serde_json::Value, serde_json::Value> for Lambda {
    fn invoke(&self, ctx: Context, input: serde_json::Value) -> RunnableFuture<serde_json::Value> {
        match &self.invoke_fn {
            Some(f) => f(ctx, input),
            None => Box::pin(async move {
                Err(RunnableError::NotImplemented(
                    "invoke not implemented".into(),
                ))
            }),
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

pub fn invokable_lambda<F, Fut>(
    name: &str,
    f: F,
) -> Arc<dyn Runnable<serde_json::Value, serde_json::Value>>
where
    F: Fn(Context, serde_json::Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = RunnableResult<serde_json::Value>> + Send + 'static,
{
    Arc::new(Lambda::invokable(name, f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lambda_invoke() {
        let lambda = Lambda::invokable(
            "upper",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("");
                Ok(serde_json::Value::String(s.to_uppercase()))
            },
        );

        let ctx = Context::new();
        let result = lambda
            .invoke(ctx, serde_json::json!("hello"))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!("HELLO"));
    }

    #[tokio::test]
    async fn test_lambda_name() {
        let lambda = Lambda::invokable(
            "my_lambda",
            |_ctx: Context, input: serde_json::Value| async move { Ok(input) },
        );
        assert_eq!(lambda.name(), "my_lambda");
    }

    #[tokio::test]
    async fn test_invokable_lambda_helper() {
        let lambda = invokable_lambda(
            "add_prefix",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("");
                Ok(serde_json::Value::String(format!("prefix_{}", s)))
            },
        );

        let ctx = Context::new();
        let result = lambda.invoke(ctx, serde_json::json!("test")).await.unwrap();
        assert_eq!(result, serde_json::json!("prefix_test"));
    }
}
