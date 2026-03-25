use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock as TokioRwLock;

pub struct RunInfo {
    pub name: String,
    pub started_at: Instant,
    pub metadata: std::collections::HashMap<String, String>,
}

impl RunInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            started_at: Instant::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    pub fn duration(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

#[async_trait]
pub trait CallbackHandler: Send + Sync {
    async fn on_start(&self, info: &RunInfo);
    async fn on_end(&self, info: &RunInfo);
    async fn on_error(&self, info: &RunInfo, error: &dyn std::error::Error);
}

pub struct Context {
    handlers: Arc<TokioRwLock<Vec<Arc<dyn CallbackHandler>>>>,
    data: Arc<tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(TokioRwLock::new(Vec::new())),
            data: Arc::new(TokioRwLock::new(std::collections::HashMap::new())),
        }
    }
    
    pub fn with_handler(handler: impl CallbackHandler + 'static) -> Self {
        let ctx = Self::new();
        ctx.add_handler(Arc::new(handler));
        ctx
    }
    
    pub fn add_handler(&self, handler: Arc<dyn CallbackHandler>) {
        // Use blocking_write for synchronous context (from non-async code)
        let mut guard = futures::executor::block_on(self.handlers.write());
        guard.push(handler);
    }
    
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.data.read().await.get(key).cloned()
    }
    
    pub async fn set(&self, key: impl Into<String>, value: serde_json::Value) {
        self.data.write().await.insert(key.into(), value);
    }
    
    pub async fn notify_start(&self, info: &RunInfo) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            handler.on_start(info).await;
        }
    }
    
    pub async fn notify_end(&self, info: &RunInfo) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            handler.on_end(info).await;
        }
    }
    
    pub async fn notify_error(&self, info: &RunInfo, error: &dyn std::error::Error) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            handler.on_error(info, error).await;
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
