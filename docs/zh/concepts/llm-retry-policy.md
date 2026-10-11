---
title: LLM Retry Policy
---

# LLM 重试策略

为 LLM 调用提供可配置的指数退避重试。

## 快速开始

```rust
use synapse::providers::{RetryPolicy, RetryMode, BackoffConfig};

let policy = RetryPolicy::new()
    .with_max_retries(3);

let result = policy.execute(|| async {
    provider.chat(messages, options).await
}).await?;
```

## 配置

### 默认策略

```rust
let policy = RetryPolicy::default();
// - mode: Normal
// - max_retries: 2
// - retryable_codes: RATE_LIMIT, SERVER, TIMEOUT, TRANSPORT
// - backoff: 500ms initial, 10s max, 0.1 jitter
```

### 自定义策略

```rust
let policy = RetryPolicy::new()
    .with_mode(RetryMode::Always)
    .with_max_retries(5)
    .with_retryable_codes(vec![
        "RATE_LIMIT".to_string(),
        "SERVER".to_string(),
    ])
    .with_backoff(BackoffConfig {
        initial_delay_ms: 1000,
        max_delay_ms: 30000,
        jitter_ratio: 0.2,
    });
```

## 重试模式

| 模式 | 行为 |
|------|----------|
| `Normal` | 仅重试配置过的错误码 |
| `Always` | 重试所有失败 |

## 退避算法

```
delay = initial_delay * 2^attempt
actual_delay = delay + (delay * jitter_ratio * random())
final_delay = min(actual_delay, max_delay)
```

## 错误码

| 错误码 | 可重试 | 说明 |
|------|-----------|-------------|
| `RATE_LIMIT` | ✅ | 429 请求过多 |
| `SERVER` | ✅ | 500/502/503 |
| `TIMEOUT` | ✅ | 请求超时 |
| `TRANSPORT` | ✅ | 网络错误 |
| `UNKNOWN` | ❌ | 其他错误 |
