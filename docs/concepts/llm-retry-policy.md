---
title: LLM Retry Policy
---

# LLM Retry Policy

Configurable retry with exponential backoff for LLM calls.

## Quick Start

```rust
use synapse::providers::{RetryPolicy, RetryMode, BackoffConfig};

let policy = RetryPolicy::new()
    .with_max_retries(3);

let result = policy.execute(|| async {
    provider.chat(messages, options).await
}).await?;
```

## Configuration

### Default Policy

```rust
let policy = RetryPolicy::default();
// - mode: Normal
// - max_retries: 2
// - retryable_codes: RATE_LIMIT, SERVER, TIMEOUT, TRANSPORT
// - backoff: 500ms initial, 10s max, 0.1 jitter
```

### Custom Policy

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

## Retry Modes

| Mode | Behavior |
|------|----------|
| `Normal` | Only retry configured error codes |
| `Always` | Retry all failures |

## Backoff Algorithm

```
delay = initial_delay * 2^attempt
actual_delay = delay + (delay * jitter_ratio * random())
final_delay = min(actual_delay, max_delay)
```

## Error Codes

| Code | Retryable | Description |
|------|-----------|-------------|
| `RATE_LIMIT` | ✅ | 429 Too Many Requests |
| `SERVER` | ✅ | 500/502/503 |
| `TIMEOUT` | ✅ | Request timeout |
| `TRANSPORT` | ✅ | Network error |
| `UNKNOWN` | ❌ | Other errors |
