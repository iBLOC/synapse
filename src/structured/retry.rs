use std::marker::PhantomData;
use std::sync::Arc;

use crate::structured::{OutputError, OutputParser, OutputResult};

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: usize,
    /// 初始退避时间（毫秒）
    pub initial_backoff_ms: u64,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// 最大退避时间（毫秒）
    pub max_backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 5000,
        }
    }
}

/// 带重试的解析器包装器
///
/// 当解析失败时，使用重试函数重新调用 LLM。
pub struct RetryParser<T, F>
where
    F: Fn(String) -> Result<String, OutputError> + Send + Sync,
{
    inner: Arc<dyn OutputParser<T> + Send + Sync>,
    config: RetryConfig,
    retry_fn: Arc<F>,
    _phantom: PhantomData<T>,
}

impl<T, F> RetryParser<T, F>
where
    T: Send + Sync + 'static,
    F: Fn(String) -> Result<String, OutputError> + Send + Sync,
{
    pub fn new(
        inner: Arc<dyn OutputParser<T> + Send + Sync>,
        retry_fn: Arc<F>,
    ) -> Self {
        Self {
            inner,
            config: RetryConfig::default(),
            retry_fn,
            _phantom: PhantomData,
        }
    }

    pub fn with_config(
        inner: Arc<dyn OutputParser<T> + Send + Sync>,
        retry_fn: Arc<F>,
        config: RetryConfig,
    ) -> Self {
        Self {
            inner,
            config,
            retry_fn,
            _phantom: PhantomData,
        }
    }

    /// 带重试的解析
    pub async fn parse_with_retry(&self, initial_text: &str) -> OutputResult<T> {
        let mut current_text = initial_text.to_string();
        let mut backoff = self.config.initial_backoff_ms;

        for attempt in 0..=self.config.max_retries {
            match self.inner.parse(&current_text) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    if attempt == self.config.max_retries {
                        return Err(OutputError::MaxRetries(e.to_string()));
                    }

                    let error_msg = self.inner.format_error_for_retry(&current_text, &e);
                    match (self.retry_fn)(error_msg) {
                        Ok(new_text) => {
                            current_text = new_text;
                            tokio_sleep(backoff).await;
                            backoff = ((backoff as f64) * self.config.backoff_multiplier) as u64;
                            backoff = backoff.min(self.config.max_backoff_ms);
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }

        Err(OutputError::MaxRetries("max retries reached".into()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn tokio_sleep(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[cfg(target_arch = "wasm32")]
async fn tokio_sleep(ms: u64) {
    let _ = ms;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::OutputError;
    use crate::structured::json::JsonOutputParser;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Person {
        name: String,
    }

#[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let retry_fn = Arc::new(move |_error_msg: String| -> Result<String, OutputError> {
            let n = attempts_clone.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Ok(r#"{"name": "Alice"}"#.to_string())
            } else {
                Ok("still bad".to_string())
            }
        });

        let parser = JsonOutputParser::<Person>::new();
        let retry_parser = RetryParser::new(Arc::new(parser), retry_fn);

        let result = retry_parser
            .parse_with_retry("not json at all")
            .await
            .unwrap();
        assert_eq!(result.name, "Alice");
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let retry_fn = Arc::new(|_error: String| -> Result<String, OutputError> {
            Ok("not json at all".to_string())
        });

        let parser = JsonOutputParser::<Person>::new();
        let retry_parser = RetryParser::new(Arc::new(parser), retry_fn);

        let result = retry_parser
            .parse_with_retry("not json")
            .await;
        assert!(result.is_err());
    }
}