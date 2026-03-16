use std::sync::Arc;

pub type CallbackResult = Result<(), CallbackError>;

#[derive(Debug, thiserror::Error)]
pub enum CallbackError {
    #[error("callback error: {0}")]
    Callback(String),
}

pub trait CallbackHandler: Send + Sync {
    fn on_start(&self, _ctx: &CallbackContext) -> CallbackResult {
        Ok(())
    }
    fn on_end(&self, _ctx: &CallbackContext, _result: &CallbackResult) -> CallbackResult {
        Ok(())
    }
    fn on_error(&self, _ctx: &CallbackContext, _error: &CallbackError) -> CallbackResult {
        Ok(())
    }
}

pub struct CallbackContext {
    pub component: String,
    pub component_type: ComponentType,
    pub run_id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ComponentType {
    ChatModel,
    Tool,
    Retriever,
    Embedding,
    Agent,
    Graph,
}

pub struct CallbackManager {
    handlers: Vec<Arc<dyn CallbackHandler>>,
}

impl CallbackManager {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Arc<dyn CallbackHandler>) {
        self.handlers.push(handler);
    }

    pub fn on_start(&self, ctx: &CallbackContext) {
        for handler in &self.handlers {
            let _ = handler.on_start(ctx);
        }
    }
}

impl Default for CallbackManager {
    fn default() -> Self {
        Self::new()
    }
}
