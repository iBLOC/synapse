use std::sync::Arc;

pub mod observability;

#[cfg(feature = "opentelemetry")]
pub mod opentelemetry_exporter;

pub use observability::{
    ActiveSpan, MemoryExporter, Metrics, MetricsHandler, MetricsSnapshot, Span, SpanExporter,
    SpanStatus, StdoutExporter, Tracer, TracerHandler,
};

#[cfg(feature = "opentelemetry")]
pub use opentelemetry_exporter::{
    init_otel_provider, init_otel_provider_with_attrs, set_global_provider, OtelError,
    OtelSpanExporter,
};

pub type CallbackResult = Result<(), CallbackError>;

#[derive(Debug, thiserror::Error)]
pub enum CallbackError {
    #[error("callback error: {0}")]
    Callback(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CallbackTiming {
    OnStart = 0,
    OnEnd = 1,
    OnError = 2,
    OnStartWithStreamInput = 3,
    OnEndWithStreamOutput = 4,
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
    Chain,
    Workflow,
    Lambda,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Invoke,
    Stream,
    Collect,
    Transform,
}

#[derive(Debug, Clone)]
pub struct RunInfo {
    pub name: String,
    pub run_type: RunType,
    pub component: ComponentType,
    pub component_name: String,
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

#[derive(Debug)]
pub struct CallbackInput(pub serde_json::Value);

#[derive(Debug)]
pub struct CallbackOutput(pub serde_json::Value);

pub trait TimingChecker: Send + Sync {
    fn needed(&self, info: &RunInfo, timing: CallbackTiming) -> bool;
}

impl<T: Handler + Send + Sync> TimingChecker for T {
    fn needed(&self, _info: &RunInfo, _timing: CallbackTiming) -> bool {
        true
    }
}

pub trait Handler: Send + Sync {
    fn on_start(&self, _info: &RunInfo, _input: &serde_json::Value) {}
    fn on_end(&self, _info: &RunInfo, _output: &serde_json::Value) {}
    fn on_error(&self, _info: &RunInfo, _error: &str) {}
    fn on_start_with_stream_input(&self, _info: &RunInfo, _input: &serde_json::Value) {
        // see on_end_with_stream_output
    }
    fn on_end_with_stream_output(&self, _info: &RunInfo, _output: &serde_json::Value) {}
}

pub struct HandlerBuilder {
    on_start_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_error_fn: Option<Box<dyn Fn(&RunInfo, &str) + Send + Sync>>,
    on_start_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
}

impl HandlerBuilder {
    pub fn new() -> Self {
        Self {
            on_start_fn: None,
            on_end_fn: None,
            on_error_fn: None,
            on_start_stream_fn: None,
            on_end_stream_fn: None,
        }
    }

    pub fn on_start<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_start_fn = Some(Box::new(f));
        self
    }

    pub fn on_end<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_end_fn = Some(Box::new(f));
        self
    }

    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &str) + Send + Sync + 'static,
    {
        self.on_error_fn = Some(Box::new(f));
        self
    }

    pub fn on_start_with_stream_input<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_start_stream_fn = Some(Box::new(f));
        self
    }

    pub fn on_end_with_stream_output<F>(mut self, f: F) -> Self
    where
        F: Fn(&RunInfo, &serde_json::Value) + Send + Sync + 'static,
    {
        self.on_end_stream_fn = Some(Box::new(f));
        self
    }

    pub fn build(self) -> BuiltHandler {
        BuiltHandler {
            on_start_fn: self.on_start_fn,
            on_end_fn: self.on_end_fn,
            on_error_fn: self.on_error_fn,
            on_start_stream_fn: self.on_start_stream_fn,
            on_end_stream_fn: self.on_end_stream_fn,
        }
    }
}

impl Default for HandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BuiltHandler {
    on_start_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_error_fn: Option<Box<dyn Fn(&RunInfo, &str) + Send + Sync>>,
    on_start_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
    on_end_stream_fn: Option<Box<dyn Fn(&RunInfo, &serde_json::Value) + Send + Sync>>,
}

impl Handler for BuiltHandler {
    fn on_start(&self, info: &RunInfo, input: &serde_json::Value) {
        if let Some(ref f) = self.on_start_fn {
            f(info, input);
        }
    }

    fn on_end(&self, info: &RunInfo, output: &serde_json::Value) {
        if let Some(ref f) = self.on_end_fn {
            f(info, output);
        }
    }

    fn on_error(&self, info: &RunInfo, error: &str) {
        if let Some(ref f) = self.on_error_fn {
            f(info, error);
        }
    }

    fn on_start_with_stream_input(&self, info: &RunInfo, input: &serde_json::Value) {
        if let Some(ref f) = self.on_start_stream_fn {
            f(info, input);
        } else {
            self.on_start(info, input);
        }
    }

    fn on_end_with_stream_output(&self, info: &RunInfo, output: &serde_json::Value) {
        if let Some(ref f) = self.on_end_stream_fn {
            f(info, output);
        } else {
            self.on_end(info, output);
        }
    }
}

#[derive(Clone, Default)]
pub struct CallbackManager {
    global_handlers: Arc<Vec<Arc<dyn Handler>>>,
    handlers: Arc<Vec<Arc<dyn Handler>>>,
}

impl CallbackManager {
    pub fn new() -> Self {
        Self {
            global_handlers: Arc::new(Vec::new()),
            handlers: Arc::new(Vec::new()),
        }
    }

    pub fn with_handler(mut self, handler: Arc<dyn Handler>) -> Self {
        Arc::make_mut(&mut self.handlers).push(handler);
        self
    }

    pub fn with_global(mut self, handler: Arc<dyn Handler>) -> Self {
        Arc::make_mut(&mut self.global_handlers).push(handler);
        self
    }

    fn all_handlers(&self) -> Vec<Arc<dyn Handler>> {
        let mut all = self.global_handlers.as_ref().clone();
        all.extend(self.handlers.as_ref().clone());
        all
    }

    pub fn run_start(&self, info: &RunInfo, input: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_start(info, input);
        }
    }

    pub fn run_end(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_end(info, output);
        }
    }

    pub fn run_error(&self, info: &RunInfo, error: &str) {
        for h in self.all_handlers() {
            h.on_error(info, error);
        }
    }

    pub fn run_start_with_stream_input(&self, info: &RunInfo, input: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_start_with_stream_input(info, input);
        }
    }

    pub fn run_end_with_stream_output(&self, info: &RunInfo, output: &serde_json::Value) {
        for h in self.all_handlers() {
            h.on_end_with_stream_output(info, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopHandler;
    impl Handler for NoopHandler {}

    #[test]
    fn test_handler_builder_on_start() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLED: AtomicUsize = AtomicUsize::new(0);

        let handler = HandlerBuilder::new()
            .on_start(|_, _| {
                CALLED.fetch_add(1, Ordering::SeqCst);
            })
            .build();

        let info = RunInfo::new("test", RunType::Invoke, ComponentType::Graph, "g");
        handler.on_start(&info, &serde_json::json!({}));
        assert_eq!(CALLED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handler_builder_all_timings() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLED: AtomicUsize = AtomicUsize::new(0);

        let handler = HandlerBuilder::new()
            .on_start(|_, _| {
                CALLED.fetch_add(1, Ordering::SeqCst);
            })
            .on_end(|_, _| {
                CALLED.fetch_add(10, Ordering::SeqCst);
            })
            .on_error(|_, _| {
                CALLED.fetch_add(100, Ordering::SeqCst);
            })
            .build();

        let info = RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g");
        handler.on_start(&info, &serde_json::json!({}));
        assert_eq!(CALLED.load(Ordering::SeqCst), 1);
        handler.on_end(&info, &serde_json::json!({}));
        assert_eq!(CALLED.load(Ordering::SeqCst), 11);
        handler.on_error(&info, "err");
        assert_eq!(CALLED.load(Ordering::SeqCst), 111);
    }

    #[test]
    fn test_timing_checker_returns_correct() {
        // SelectiveHandler does NOT impl Handler, so it gets its own TimingChecker impl
        struct SelectiveHandler;
        impl TimingChecker for SelectiveHandler {
            fn needed(&self, _info: &RunInfo, timing: CallbackTiming) -> bool {
                timing == CallbackTiming::OnStart
            }
        }

        let h = SelectiveHandler;
        let info = RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g");
        assert!(h.needed(&info, CallbackTiming::OnStart));
        assert!(!h.needed(&info, CallbackTiming::OnEnd));
    }

    #[test]
    fn test_callback_manager_global_and_local() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static GLOBAL: AtomicUsize = AtomicUsize::new(0);
        static LOCAL: AtomicUsize = AtomicUsize::new(0);

        let global = HandlerBuilder::new()
            .on_start(|_, _| {
                GLOBAL.fetch_add(1, Ordering::SeqCst);
            })
            .build();
        let local = HandlerBuilder::new()
            .on_start(|_, _| {
                LOCAL.fetch_add(1, Ordering::SeqCst);
            })
            .build();

        let manager = CallbackManager::new()
            .with_global(Arc::new(global))
            .with_handler(Arc::new(local));

        let info = RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g");
        manager.run_start(&info, &serde_json::json!({}));

        assert_eq!(GLOBAL.load(Ordering::SeqCst), 1);
        assert_eq!(LOCAL.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_run_info_new() {
        let info = RunInfo::new(
            "node1",
            RunType::Invoke,
            ComponentType::ChatModel,
            "my_model",
        );
        assert_eq!(info.name, "node1");
        assert_eq!(info.run_type, RunType::Invoke);
        assert_eq!(info.component, ComponentType::ChatModel);
        assert_eq!(info.component_name, "my_model");
    }

    #[test]
    fn test_callback_manager_run_end_and_error() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static END_CALLED: AtomicUsize = AtomicUsize::new(0);
        static ERROR_CALLED: AtomicUsize = AtomicUsize::new(0);

        let end_handler = HandlerBuilder::new()
            .on_end(|_, _| {
                END_CALLED.fetch_add(1, Ordering::SeqCst);
            })
            .build();
        let error_handler = HandlerBuilder::new()
            .on_error(|_, _| {
                ERROR_CALLED.fetch_add(1, Ordering::SeqCst);
            })
            .build();

        let manager = CallbackManager::new()
            .with_handler(Arc::new(end_handler))
            .with_handler(Arc::new(error_handler));

        let info = RunInfo::new("t", RunType::Invoke, ComponentType::Tool, "tool");
        manager.run_end(&info, &serde_json::json!({"result": "ok"}));
        manager.run_error(&info, "something went wrong");

        assert_eq!(END_CALLED.load(Ordering::SeqCst), 1);
        assert_eq!(ERROR_CALLED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handler_default_stream_delegation() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static ON_START: AtomicUsize = AtomicUsize::new(0);
        static ON_END: AtomicUsize = AtomicUsize::new(0);

        let handler = HandlerBuilder::new()
            .on_start(|_, _| {
                ON_START.fetch_add(1, Ordering::SeqCst);
            })
            .on_end(|_, _| {
                ON_END.fetch_add(1, Ordering::SeqCst);
            })
            .build();

        let info = RunInfo::new("t", RunType::Invoke, ComponentType::Graph, "g");
        let val = serde_json::json!({});

        handler.on_start_with_stream_input(&info, &val);
        handler.on_end_with_stream_output(&info, &val);

        assert_eq!(ON_START.load(Ordering::SeqCst), 1);
        assert_eq!(ON_END.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_noop_handler_implements_handler() {
        let _h: NoopHandler = NoopHandler;
    }
}
