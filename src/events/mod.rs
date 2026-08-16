use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventError {
    #[error("listener error: {0}")]
    Listener(String),
    #[error("no listeners for event: {0}")]
    NoListeners(String),
    #[error("max retries exceeded")]
    MaxRetriesExceeded,
}

pub type EventResult<T> = Result<T, EventError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchMode {
    Emit,
    Parallel,
    Serial,
    Bail,
    Waterfall,
}

pub struct Next {
    inner: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = EventResult<()>> + Send>> + Send>,
}

impl Next {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = EventResult<()>> + Send + 'static,
    {
        Self {
            inner: Box::new(move || Box::pin(f())),
        }
    }

    pub async fn call(self) -> EventResult<()> {
        (self.inner)().await
    }
}

#[async_trait::async_trait]
pub trait WaterfallListener: Send + Sync {
    async fn handle(&self, event: &mut EventData, next: Next) -> EventResult<()>;
}

pub struct EventData {
    pub name: String,
    pub payload: serde_json::Value,
    pub handled: bool,
}

impl EventData {
    pub fn new(name: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            payload,
            handled: false,
        }
    }
}

struct ListenerEntry {
    listener: Arc<dyn WaterfallListener>,
    priority: i32,
}

pub struct EventBus {
    listeners: RwLock<HashMap<String, Vec<ListenerEntry>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(HashMap::new()),
        }
    }

    pub fn on_waterfall(
        &self,
        event_name: &str,
        listener: Arc<dyn WaterfallListener>,
        priority: i32,
    ) {
        let mut listeners = self.listeners.write();
        let entry = ListenerEntry { listener, priority };
        let list = listeners.entry(event_name.to_string()).or_insert_with(Vec::new);
        list.push(entry);
        list.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub async fn dispatch_waterfall(
        &self,
        event_name: &str,
        mut event: EventData,
    ) -> EventResult<()> {
        let listeners: Vec<Arc<dyn WaterfallListener>> = {
            let map = self.listeners.read();
            map.get(event_name)
                .map(|list| list.iter().map(|e| e.listener.clone()).collect())
                .unwrap_or_default()
        };

        if listeners.is_empty() {
            return Ok(());
        }

        let mut iter = listeners.into_iter().rev();
        let first = iter.next().unwrap();

        let next = self.build_chain(iter, &mut event);
        first.handle(&mut event, next).await
    }

    fn build_chain(
        &self,
        iter: impl Iterator<Item = Arc<dyn WaterfallListener>> + Send + 'static,
        _event: &mut EventData,
    ) -> Next {
        let remaining: Vec<Arc<dyn WaterfallListener>> = iter.collect();

        if remaining.is_empty() {
            return Next::new(|| async { Ok(()) });
        }

        let mut remaining = remaining;
        remaining.reverse();

        Self::build_chain_recursive(remaining)
    }

    fn build_chain_recursive(
        mut listeners: Vec<Arc<dyn WaterfallListener>>,
    ) -> Next {
        if listeners.is_empty() {
            return Next::new(|| async { Ok(()) });
        }

        let current = listeners.remove(0);
        let next = Self::build_chain_recursive(listeners);

        Next::new(move || async move {
            current.handle(&mut EventData::new("", serde_json::json!({})), next).await
        })
    }

    pub async fn dispatch_emit(&self, event_name: &str, event: &EventData) -> EventResult<()> {
        let listeners: Vec<Arc<dyn WaterfallListener>> = {
            let map = self.listeners.read();
            map.get(event_name)
                .map(|list| list.iter().map(|e| e.listener.clone()).collect())
                .unwrap_or_default()
        };

        for listener in listeners {
            let mut e = EventData::new(&event.name, event.payload.clone());
            let next = Next::new(|| async { Ok(()) });
            listener.handle(&mut e, next).await?;
        }

        Ok(())
    }

    pub async fn dispatch_parallel(&self, event_name: &str, event: &EventData) -> EventResult<()> {
        let listeners: Vec<Arc<dyn WaterfallListener>> = {
            let map = self.listeners.read();
            map.get(event_name)
                .map(|list| list.iter().map(|e| e.listener.clone()).collect())
                .unwrap_or_default()
        };

        let mut handles = Vec::new();
        for listener in listeners {
            let mut e = EventData::new(&event.name, event.payload.clone());
            let next = Next::new(|| async { Ok(()) });
            handles.push(tokio::spawn(async move {
                listener.handle(&mut e, next).await
            }));
        }

        for handle in handles {
            handle.await.map_err(|e| EventError::Listener(e.to_string()))??;
        }

        Ok(())
    }

    pub async fn dispatch_serial(&self, event_name: &str, event: &EventData) -> EventResult<()> {
        let listeners: Vec<Arc<dyn WaterfallListener>> = {
            let map = self.listeners.read();
            map.get(event_name)
                .map(|list| list.iter().map(|e| e.listener.clone()).collect())
                .unwrap_or_default()
        };

        for listener in listeners {
            let mut e = EventData::new(&event.name, event.payload.clone());
            let next = Next::new(|| async { Ok(()) });
            listener.handle(&mut e, next).await?;
        }

        Ok(())
    }

    pub fn listener_count(&self, event_name: &str) -> usize {
        let map = self.listeners.read();
        map.get(event_name).map(|list| list.len()).unwrap_or(0)
    }

    pub fn event_names(&self) -> Vec<String> {
        let map = self.listeners.read();
        map.keys().cloned().collect()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TestListener {
        name: String,
        counter: Arc<AtomicU32>,
    }

    impl TestListener {
        fn new(name: &str, counter: Arc<AtomicU32>) -> Self {
            Self {
                name: name.to_string(),
                counter,
            }
        }
    }

    #[async_trait::async_trait]
    impl WaterfallListener for TestListener {
        async fn handle(&self, event: &mut EventData, next: Next) -> EventResult<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            next.call().await
        }
    }

    #[tokio::test]
    async fn test_waterfall_dispatch() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        bus.on_waterfall("test", Arc::new(TestListener::new("a", counter.clone())), 10);
        bus.on_waterfall("test", Arc::new(TestListener::new("b", counter.clone())), 5);

        let event = EventData::new("test", serde_json::json!({}));
        bus.dispatch_waterfall("test", event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_emit_dispatch() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        bus.on_waterfall("test", Arc::new(TestListener::new("a", counter.clone())), 10);
        bus.on_waterfall("test", Arc::new(TestListener::new("b", counter.clone())), 5);

        let event = EventData::new("test", serde_json::json!({}));
        bus.dispatch_emit("test", &event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_parallel_dispatch() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        bus.on_waterfall("test", Arc::new(TestListener::new("a", counter.clone())), 10);
        bus.on_waterfall("test", Arc::new(TestListener::new("b", counter.clone())), 5);

        let event = EventData::new("test", serde_json::json!({}));
        bus.dispatch_parallel("test", &event).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_no_listeners() {
        let bus = EventBus::new();
        let event = EventData::new("test", serde_json::json!({}));
        let result = bus.dispatch_waterfall("test", event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_listener_count() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicU32::new(0));

        assert_eq!(bus.listener_count("test"), 0);

        bus.on_waterfall("test", Arc::new(TestListener::new("a", counter.clone())), 10);
        assert_eq!(bus.listener_count("test"), 1);

        bus.on_waterfall("test", Arc::new(TestListener::new("b", counter.clone())), 5);
        assert_eq!(bus.listener_count("test"), 2);
    }
}
