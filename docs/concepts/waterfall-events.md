---
title: Waterfall Events
---

# Waterfall Events

Interceptor-pattern event system with 5 dispatch modes, inspired by Cordis.

## Dispatch Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `emit` | Concurrent, ignore returns | Fire-and-forget notifications |
| `parallel` | Concurrent, await all | Independent side effects |
| `serial` | Sequential, await each | Ordered processing |
| `bail` | Stop on first non-null | Early termination |
| `waterfall` | Chain with `next()` | LLM request interceptors |

## Usage

### Basic Waterfall

```rust
use synapse::events::*;

struct RetryListener;

#[async_trait]
impl WaterfallListener for RetryListener {
    async fn handle(&self, event: &mut EventData, next: Next) -> EventResult<()> {
        for attempt in 0..3 {
            match next.call().await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempt))).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(EventError::MaxRetriesExceeded)
    }
}

let bus = EventBus::new();
bus.on_waterfall("llm/stream", Arc::new(RetryListener), 10);
```

### Multiple Listeners

```rust
// Higher priority runs first
bus.on_waterfall("request", Arc::new(AuthListener), 100);  // Auth first
bus.on_waterfall("request", Arc::new(LogListener), 50);    // Then logging
bus.on_waterfall("request", Arc::new(RetryListener), 10);  // Then retry

let event = EventData::new("request", serde_json::json!({}));
bus.dispatch_waterfall("request", event).await?;
```

### Dispatch

```rust
// Waterfall (chain with next)
bus.dispatch_waterfall("event", event).await?;

// Parallel (concurrent)
bus.dispatch_parallel("event", &event).await?;

// Serial (sequential)
bus.dispatch_serial("event", &event).await?;
```
