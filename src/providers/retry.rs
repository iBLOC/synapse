use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::LLMError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub mode: RetryMode,
    pub max_retries: u32,
    pub retryable_codes: Vec<String>,
    pub backoff: BackoffConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryMode {
    Normal,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffConfig {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_ratio: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            mode: RetryMode::Normal,
            max_retries: 2,
            retryable_codes: vec![
                "RATE_LIMIT".to_string(),
                "SERVER".to_string(),
                "TIMEOUT".to_string(),
                "TRANSPORT".to_string(),
            ],
            backoff: BackoffConfig::default(),
        }
    }
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 500,
            max_delay_ms: 10_000,
            jitter_ratio: 0.1,
        }
    }
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mode(mut self, mode: RetryMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_retryable_codes(mut self, codes: Vec<String>) -> Self {
        self.retryable_codes = codes;
        self
    }

    pub fn with_backoff(mut self, backoff: BackoffConfig) -> Self {
        self.backoff = backoff;
        self
    }

    pub async fn execute<F, Fut, T>(&self, mut f: F) -> Result<T, LLMError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, LLMError>>,
    {
        let mut delay = self.backoff.initial_delay_ms;

        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if self.should_retry(&e, attempt) => {
                    let jitter = delay as f64 * self.backoff.jitter_ratio;
                    let actual_delay = delay + (jitter * rand::random::<f64>()) as u64;
                    tokio::time::sleep(Duration::from_millis(actual_delay)).await;
                    delay = (delay * 2).min(self.backoff.max_delay_ms);
                }
                Err(e) => return Err(e),
            }
        }

        Err(LLMError::Api("max retries exceeded".to_string()))
    }

    fn should_retry(&self, error: &LLMError, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }

        match self.mode {
            RetryMode::Normal => {
                let code = self.error_code(error);
                self.retryable_codes.contains(&code)
            }
            RetryMode::Always => true,
        }
    }

    fn error_code(&self, error: &LLMError) -> String {
        match error {
            LLMError::RateLimit(_) => "RATE_LIMIT".to_string(),
            LLMError::Timeout => "TIMEOUT".to_string(),
            LLMError::Api(msg) if msg.contains("500") || msg.contains("502") || msg.contains("503") => {
                "SERVER".to_string()
            }
            LLMError::StreamError(_) => "TRANSPORT".to_string(),
            _ => "UNKNOWN".to_string(),
        }
    }
}

pub fn exponential_backoff(attempt: u32, initial_ms: u64, max_ms: u64) -> Duration {
    let delay = initial_ms * 2u64.pow(attempt);
    Duration::from_millis(delay.min(max_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let policy = RetryPolicy::new();
        let result = policy.execute(|| async { Ok::<_, LLMError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failure() {
        let policy = RetryPolicy::new().with_max_retries(3);
        let attempt = std::sync::atomic::AtomicU32::new(0);

        let result = policy.execute(|| {
            let attempt = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if attempt < 2 {
                    Err(LLMError::Timeout)
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let policy = RetryPolicy::new().with_max_retries(2);
        let result = policy.execute(|| async {
            Err::<i32, _>(LLMError::Timeout)
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let policy = RetryPolicy::new().with_max_retries(3);
        let result = policy.execute(|| async {
            Err::<i32, _>(LLMError::InvalidRequest("bad request".to_string()))
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_always_mode() {
        let policy = RetryPolicy::new()
            .with_mode(RetryMode::Always)
            .with_max_retries(3);

        let result = policy.execute(|| async {
            Err::<i32, _>(LLMError::InvalidRequest("bad".to_string()))
        }).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_exponential_backoff() {
        let delay = exponential_backoff(0, 500, 10000);
        assert_eq!(delay, Duration::from_millis(500));

        let delay = exponential_backoff(1, 500, 10000);
        assert_eq!(delay, Duration::from_millis(1000));

        let delay = exponential_backoff(10, 500, 10000);
        assert_eq!(delay, Duration::from_millis(10000));
    }
}
