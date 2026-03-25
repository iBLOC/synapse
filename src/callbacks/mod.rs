use std::sync::Arc;

pub type CallbackResult = Result<(), CallbackError>;

#[derive(Debug, thiserror::Error)]
pub enum CallbackError {
    #[error("callback error: {0}")]
    Callback(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    ChatModel,
    Tool,
    Retriever,
    Embedding,
    Indexer,
    DocumentLoader,
    Agent,
    Graph,
    Custom,
}

#[derive(Debug, Clone)]
pub struct RunInfo {
    pub name: String,
    pub run_type: RunType,
    pub component: ComponentType,
    pub component_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Invoke,
    Stream,
    Collect,
    Transform,
}

impl RunInfo {
    pub fn new(
        name: &str,
        run_type: RunType,
        component: ComponentType,
        component_name: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            run_type,
            component,
            component_name: component_name.to_string(),
        }
    }
}

pub trait CallbackHandler: Send + Sync {
    fn on_start(&self, _info: &RunInfo) {}
    fn on_end(&self, _info: &RunInfo, _output: &serde_json::Value) {}
    fn on_error(&self, _info: &RunInfo, _error: &str) {}
    fn on_retry(&self, _info: &RunInfo, _error: &str, _attempt: u32) {}

    fn on_chat_model_start(&self, _info: &RunInfo, _messages: &[serde_json::Value]) {}
    fn on_chat_model_end(&self, _info: &RunInfo, _output: &serde_json::Value) {}

    fn on_tool_start(&self, _info: &RunInfo, _tool_name: &str, _tool_input: &str) {}
    fn on_tool_end(&self, _info: &RunInfo, _tool_name: &str, _output: &serde_json::Value) {}
    fn on_tool_error(&self, _info: &RunInfo, _tool_name: &str, _error: &str) {}

    fn on_retriever_start(&self, _info: &RunInfo, _query: &str) {}
    fn on_retriever_end(&self, _info: &RunInfo, _documents: &[serde_json::Value]) {}

    fn on_embedding_start(&self, _info: &RunInfo, _texts: &[String]) {}
    fn on_embedding_end(&self, _info: &RunInfo, _embeddings: &[Vec<f32>]) {}
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

    pub fn handlers(&self) -> &[Arc<dyn CallbackHandler>] {
        &self.handlers
    }

    pub fn run_start(&self, info: &RunInfo) {
        for h in &self.handlers {
            h.on_start(info);
        }
    }

    pub fn run_end(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in &self.handlers {
            h.on_end(info, output);
        }
    }

    pub fn run_error(&self, info: &RunInfo, error: &str) {
        for h in &self.handlers {
            h.on_error(info, error);
        }
    }

    pub fn run_retry(&self, info: &RunInfo, error: &str, attempt: u32) {
        for h in &self.handlers {
            h.on_retry(info, error, attempt);
        }
    }

    pub fn chat_model_start(&self, info: &RunInfo, messages: &[serde_json::Value]) {
        for h in &self.handlers {
            h.on_chat_model_start(info, messages);
        }
    }

    pub fn chat_model_end(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in &self.handlers {
            h.on_chat_model_end(info, output);
        }
    }

    pub fn tool_start(&self, info: &RunInfo, tool_name: &str, tool_input: &str) {
        for h in &self.handlers {
            h.on_tool_start(info, tool_name, tool_input);
        }
    }

    pub fn tool_end(&self, info: &RunInfo, tool_name: &str, output: &serde_json::Value) {
        for h in &self.handlers {
            h.on_tool_end(info, tool_name, output);
        }
    }

    pub fn tool_error(&self, info: &RunInfo, tool_name: &str, error: &str) {
        for h in &self.handlers {
            h.on_tool_error(info, tool_name, error);
        }
    }

    pub fn retriever_start(&self, info: &RunInfo, query: &str) {
        for h in &self.handlers {
            h.on_retriever_start(info, query);
        }
    }

    pub fn retriever_end(&self, info: &RunInfo, documents: &[serde_json::Value]) {
        for h in &self.handlers {
            h.on_retriever_end(info, documents);
        }
    }

    pub fn embedding_start(&self, info: &RunInfo, texts: &[String]) {
        for h in &self.handlers {
            h.on_embedding_start(info, texts);
        }
    }

    pub fn embedding_end(&self, info: &RunInfo, embeddings: &[Vec<f32>]) {
        for h in &self.handlers {
            h.on_embedding_end(info, embeddings);
        }
    }
}

impl Default for CallbackManager {
    fn default() -> Self {
        Self::new()
    }
}
