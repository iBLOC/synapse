---
title: Waterfall Events
---

# Waterfall Events · Waterfall 事件

Interceptor-pattern event system with 5 dispatch modes, inspired by Cordis.
*拦截器（interceptor）模式的事件系统，提供 5 种分发模式，灵感来自 Cordis。*

## Dispatch Modes · 分发模式

| Mode / 模式 | Behavior / 行为 | Use Case / 使用场景 |
|------|----------|----------|
| `emit` | Concurrent, ignore returns / 并发执行，忽略返回值 | Fire-and-forget notifications / 即发即忘的通知 |
| `parallel` | Concurrent, await all / 并发执行，await 全部完成 | Independent side effects / 互不依赖的副作用 |
| `serial` | Sequential, await each / 顺序执行，逐个 await | Ordered processing / 有序处理 |
| `bail` | Stop on first non-null / 遇到首个非空即停止 | Early termination / 提前终止 |
| `waterfall` | Chain with `next()` / 通过 `next()` 链式传递 | LLM request interceptors / LLM 请求拦截器 |

## Usage · 用法

### Basic Waterfall · 基本 Waterfall

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

### Multiple Listeners · 多个 Listener

```rust
// Higher priority runs first
bus.on_waterfall("request", Arc::new(AuthListener), 100);  // Auth first
bus.on_waterfall("request", Arc::new(LogListener), 50);    // Then logging
bus.on_waterfall("request", Arc::new(RetryListener), 10);  // Then retry

let event = EventData::new("request", serde_json::json!({}));
bus.dispatch_waterfall("request", event).await?;
```

### Dispatch · 分发

```rust
// Waterfall (chain with next)
bus.dispatch_waterfall("event", event).await?;

// Parallel (concurrent)
bus.dispatch_parallel("event", &event).await?;

// Serial (sequential)
bus.dispatch_serial("event", &event).await?;
```
