use std::collections::HashMap;
use std::sync::Arc;

pub trait CallbackHandler: Send + Sync {
    fn on_start(&self, info: &RunInfo);
    fn on_end(&self, info: &RunInfo, output: &serde_json::Value);
    fn on_error(&self, info: &RunInfo, err: &str);
}

#[derive(Debug, Clone)]
pub struct RunInfo {
    pub name: String,
    pub run_type: String,
    pub component: String,
}

impl RunInfo {
    pub fn new(name: &str, run_type: &str, component: &str) -> Self {
        Self { name: name.to_string(), run_type: run_type.to_string(), component: component.to_string() }
    }
}

#[derive(Clone)]
pub struct Context {
    callbacks: Arc<Vec<Arc<dyn CallbackHandler>>>,
    values: Arc<HashMap<String, serde_json::Value>>,
    checkpoint_id: Option<String>,
}

impl Default for Context {
    fn default() -> Self { Self::new() }
}

impl Context {
    pub fn new() -> Self {
        Self { callbacks: Arc::new(Vec::new()), values: Arc::new(HashMap::new()), checkpoint_id: None }
    }

    pub fn with_callbacks(self, cbs: Vec<Arc<dyn CallbackHandler>>) -> Self {
        Self { callbacks: Arc::new(cbs), values: self.values, checkpoint_id: self.checkpoint_id }
    }

    pub fn with_value(self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let mut m = (*self.values).clone();
        m.insert(key.into(), value);
        Self { callbacks: self.callbacks, values: Arc::new(m), checkpoint_id: self.checkpoint_id }
    }

    pub fn with_checkpoint_id(self, id: impl Into<String>) -> Self {
        Self { callbacks: self.callbacks, values: self.values, checkpoint_id: Some(id.into()) }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopHandler;
    impl CallbackHandler for NoopHandler {
        fn on_start(&self, _info: &RunInfo) {}
        fn on_end(&self, _info: &RunInfo, _output: &serde_json::Value) {}
        fn on_error(&self, _info: &RunInfo, _err: &str) {}
    }

    #[test]
    fn test_context_with_values() {
        let ctx = Context::new().with_value("key", serde_json::json!("value")).with_checkpoint_id("checkpoint_1");
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
        let info = RunInfo::new("test", "invoke", "Graph");
        assert_eq!(info.name, "test");
    }
}
