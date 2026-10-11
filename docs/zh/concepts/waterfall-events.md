---
title: Waterfall Events
---

# Waterfall 事件

拦截器（interceptor）模式的事件系统，提供 5 种分发模式，灵感来自 Cordis。

## 分发模式

| 模式 | 行为 | 使用场景 |
|------|----------|----------|
| `emit` | 并发执行，忽略返回值 | 即发即忘的通知 |
| `parallel` | 并发执行，await 全部完成 | 互不依赖的副作用 |
| `serial` | 顺序执行，逐个 await | 有序处理 |
| `bail` | 遇到首个非空即停止 | 提前终止 |
| `waterfall` | 通过 `next()` 链式传递 | LLM 请求拦截器 |

## 用法

### 基本 Waterfall

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

### 多个 Listener

```rust
// Higher priority runs first
bus.on_waterfall("request", Arc::new(AuthListener), 100);  // Auth first
bus.on_waterfall("request", Arc::new(LogListener), 50);    // Then logging
bus.on_waterfall("request", Arc::new(RetryListener), 10);  // Then retry

let event = EventData::new("request", serde_json::json!({}));
bus.dispatch_waterfall("request", event).await?;
```

### 分发

```rust
// Waterfall (chain with next)
bus.dispatch_waterfall("event", event).await?;

// Parallel (concurrent)
bus.dispatch_parallel("event", &event).await?;

// Serial (sequential)
bus.dispatch_serial("event", &event).await?;
```
