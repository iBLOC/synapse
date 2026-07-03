//! LCEL 风格的链式表达式语法

use super::typed_lambda::TypedLambda;
use super::{Context, Runnable, RunnableError, RunnableFuture, RunnableResult};
use std::sync::Arc;

/// JSON 输入输出 Pipe - 使用 Arc 实现，内部存储可共享引用
pub struct JsonPipe<L, R>(Arc<L>, Arc<R>);

impl<L, R> JsonPipe<L, R>
where
    L: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    pub fn new(left: Arc<L>, right: Arc<R>) -> Self {
        Self(left, right)
    }
}

impl<L, R> Runnable<serde_json::Value, serde_json::Value> for JsonPipe<L, R>
where
    L: Runnable<serde_json::Value, serde_json::Value> + Send + Sync + 'static,
    R: Runnable<serde_json::Value, serde_json::Value> + Send + Sync + 'static,
{
    fn invoke(
        &self,
        ctx: Context,
        input: serde_json::Value,
    ) -> RunnableFuture<serde_json::Value> {
        let left = self.0.clone();
        let right = self.1.clone();
        Box::pin(async move {
            let mid = left.invoke(ctx.clone(), input).await?;
            right.invoke(ctx, mid).await
        })
    }

    fn name(&self) -> String {
        format!("{} | {}", self.0.name(), self.1.name())
    }
}

/// 旧名 - 与上面 JsonPipe 等价
pub type Pipe<L, R> = JsonPipe<L, R>;

pub fn pipe<L, R>(left: Arc<L>, right: Arc<R>) -> Pipe<L, R>
where
    L: Runnable<serde_json::Value, serde_json::Value> + Send + Sync + 'static,
    R: Runnable<serde_json::Value, serde_json::Value> + Send + Sync + 'static,
{
    Pipe::new(left, right)
}

pub struct PromptTemplate {
    template: String,
}

impl PromptTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }
}

impl Runnable<serde_json::Value, serde_json::Value> for PromptTemplate {
    fn invoke(
        &self,
        _ctx: Context,
        input: serde_json::Value,
    ) -> RunnableFuture<serde_json::Value> {
        let rendered = self.template.replace("{input}", &input.to_string());
        Box::pin(async move { Ok(serde_json::Value::String(rendered)) })
    }

    fn name(&self) -> String {
        "PromptTemplate".to_string()
    }
}

pub fn prompt_template(template: impl Into<String>) -> Arc<PromptTemplate> {
    Arc::new(PromptTemplate::new(template))
}

pub struct StringOutputParser;

impl Runnable<serde_json::Value, serde_json::Value> for StringOutputParser {
    fn invoke(
        &self,
        _ctx: Context,
        input: serde_json::Value,
    ) -> RunnableFuture<serde_json::Value> {
        Box::pin(async move {
            let s = match input {
                serde_json::Value::String(s) => s,
                serde_json::Value::Object(mut obj) => {
                    match obj.remove("content") {
                        Some(serde_json::Value::String(s)) => s,
                        Some(other) => other.to_string(),
                        None => serde_json::Value::Object(obj).to_string(),
                    }
                }
                other => other.to_string(),
            };
            Ok(serde_json::Value::String(s))
        })
    }

    fn name(&self) -> String {
        "StringOutputParser".to_string()
    }
}

pub fn string_parser() -> Arc<StringOutputParser> {
    Arc::new(StringOutputParser)
}

pub struct JsonOutputParser;

impl Runnable<serde_json::Value, serde_json::Value> for JsonOutputParser {
    fn invoke(
        &self,
        _ctx: Context,
        input: serde_json::Value,
    ) -> RunnableFuture<serde_json::Value> {
        Box::pin(async move {
            let s = input.as_str().unwrap_or("{}");
            let parsed: serde_json::Value = serde_json::from_str(s)
                .map_err(|e| RunnableError::NotImplemented(format!("parse json: {e}")))?;
            Ok(parsed)
        })
    }

    fn name(&self) -> String {
        "JsonOutputParser".to_string()
    }
}

pub fn json_parser() -> Arc<JsonOutputParser> {
    Arc::new(JsonOutputParser)
}

pub struct Sequence {
    steps: Vec<Arc<dyn Runnable<serde_json::Value, serde_json::Value> + Send + Sync>>,
}

impl Sequence {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn then<R>(mut self, runnable: Arc<R>) -> Self
    where
        R: Runnable<serde_json::Value, serde_json::Value> + Send + Sync + 'static,
    {
        self.steps.push(runnable);
        self
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Runnable<serde_json::Value, serde_json::Value> for Sequence {
    fn invoke(
        &self,
        ctx: Context,
        input: serde_json::Value,
    ) -> RunnableFuture<serde_json::Value> {
        let steps = self.steps.clone();
        Box::pin(async move {
            let mut value = input;
            for step in steps {
                value = step.invoke(ctx.clone(), value).await?;
            }
            Ok(value)
        })
    }

    fn name(&self) -> String {
        format!("Sequence({} steps)", self.steps.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::runnable::Context;
    use crate::compose::runnable::Lambda;

    #[tokio::test]
    async fn test_pipe_json() {
        let upper = Lambda::invokable(
            "upper",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("").to_uppercase();
                Ok(serde_json::Value::String(s))
            },
        );

        let lower = Lambda::invokable(
            "lower",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("").to_lowercase();
                Ok(serde_json::Value::String(s))
            },
        );

        let p = pipe(Arc::new(upper), Arc::new(lower));
        let ctx = Context::new();
        let result = p
            .invoke(ctx, serde_json::json!("Hello"))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[tokio::test]
    async fn test_string_output_parser() {
        let parser = string_parser();
        let ctx = Context::new();
        let result = parser
            .invoke(ctx, serde_json::json!("hello"))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[tokio::test]
    async fn test_json_output_parser() {
        let parser = json_parser();
        let ctx = Context::new();
        let result = parser
            .invoke(ctx, serde_json::json!("{\"key\": \"value\"}"))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_sequence() {
        let step1: Arc<Lambda> = Arc::new(Lambda::invokable(
            "step1",
            |_ctx: Context, input: serde_json::Value| async move {
                Ok(serde_json::json!(input.as_i64().unwrap_or(0) + 1))
            },
        ));
        let step2: Arc<Lambda> = Arc::new(Lambda::invokable(
            "step2",
            |_ctx: Context, input: serde_json::Value| async move {
                Ok(serde_json::json!(input.as_i64().unwrap_or(0) * 2))
            },
        ));

        let seq = Sequence::new().then(step1).then(step2);
        let ctx = Context::new();
        let result = seq
            .invoke(ctx, serde_json::json!(5))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!(12));
    }

    #[test]
    fn test_pipe_name() {
        let upper = Lambda::invokable(
            "upper",
            |_ctx: Context, input: serde_json::Value| async move { Ok(input) },
        );
        let lower = Lambda::invokable(
            "lower",
            |_ctx: Context, input: serde_json::Value| async move { Ok(input) },
        );
        let p = pipe(Arc::new(upper), Arc::new(lower));
        assert!(p.name().contains("|"));
    }
}