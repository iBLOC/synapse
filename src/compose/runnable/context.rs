pub use crate::callbacks::{CallbackManager, ComponentType, Handler, RunInfo, RunType};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    callbacks: Option<CallbackManager>,
    values: Arc<HashMap<String, serde_json::Value>>,
    checkpoint_id: Option<String>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            callbacks: None,
            values: Arc::new(HashMap::new()),
            checkpoint_id: None,
        }
    }

    pub fn with_callback_manager(mut self, mgr: CallbackManager) -> Self {
        self.callbacks = Some(mgr);
        self
    }

    pub fn with_value(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let mut m = (*self.values).clone();
        m.insert(key.into(), value);
        self.values = Arc::new(m);
        self
    }

    pub fn with_checkpoint_id(mut self, id: impl Into<String>) -> Self {
        self.checkpoint_id = Some(id.into());
        self
    }

    pub fn get_value(&self, key: &str) -> Option<serde_json::Value> {
        self.values.get(key).cloned()
    }

    pub fn callbacks(&self) -> Option<&CallbackManager> {
        self.callbacks.as_ref()
    }

    pub fn checkpoint_id(&self) -> Option<&str> {
        self.checkpoint_id.as_deref()
    }

    pub fn values(&self) -> Arc<HashMap<String, serde_json::Value>> {
        self.values.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopHandler;
    impl Handler for NoopHandler {}

    #[test]
    fn test_context_with_values() {
        let ctx = Context::new()
            .with_value("key", serde_json::json!("value"))
            .with_checkpoint_id("checkpoint_1");
        assert_eq!(ctx.get_value("key"), Some(serde_json::json!("value")));
        assert_eq!(ctx.checkpoint_id(), Some("checkpoint_1"));
    }

    #[test]
    fn test_context_callbacks() {
        let handler: Arc<dyn Handler> = Arc::new(NoopHandler);
        let manager = CallbackManager::new().with_handler(handler);
        let ctx = Context::new().with_callback_manager(manager);
        assert!(ctx.callbacks().is_some());
    }

    #[test]
    fn test_run_info() {
        let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "my_graph");
        assert_eq!(info.name, "test");
        assert_eq!(info.component, ComponentType::Graph);
    }
}
