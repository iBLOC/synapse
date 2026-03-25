pub use crate::callbacks::{CallbackHandler, CallbackManager, ComponentType, RunInfo, RunType};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Context {
    callbacks: Arc<Vec<Arc<dyn CallbackHandler>>>,
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
            callbacks: Arc::new(Vec::new()),
            values: Arc::new(HashMap::new()),
            checkpoint_id: None,
        }
    }

    pub fn with_callbacks(mut self, cbs: Vec<Arc<dyn CallbackHandler>>) -> Self {
        self.callbacks = Arc::new(cbs);
        self
    }

    pub fn with_callback_manager(mut self, mgr: &CallbackManager) -> Self {
        let mut cbs: Vec<Arc<dyn CallbackHandler>> = mgr.handlers().to_vec();
        for h in self.callbacks.iter() {
            cbs.push(h.clone());
        }
        self.callbacks = Arc::new(cbs);
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

    pub fn callbacks(&self) -> Arc<Vec<Arc<dyn CallbackHandler>>> {
        self.callbacks.clone()
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
    impl CallbackHandler for NoopHandler {}

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
        let handler: Arc<dyn CallbackHandler> = Arc::new(NoopHandler);
        let ctx = Context::new().with_callbacks(vec![handler]);
        assert_eq!(ctx.callbacks().len(), 1);
    }

    #[test]
    fn test_run_info() {
        let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "my_graph");
        assert_eq!(info.name, "test");
        assert_eq!(info.component, ComponentType::Graph);
    }
}
