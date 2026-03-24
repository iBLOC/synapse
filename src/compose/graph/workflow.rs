use super::super::runnable::{Context, Runnable, RunnableError, RunnableResult};
use super::options::NodeTriggerMode;
use super::{CompiledGraph, Graph, GraphNode};
use std::marker::PhantomData;
use std::sync::Arc;

pub struct Workflow<I, O> {
    graph: Graph<I, serde_json::Value>,
    _phantom: PhantomData<O>,
}

impl<I, O> Workflow<I, O> {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(NodeTriggerMode::AllPredecessor),
            _phantom: PhantomData,
        }
    }

    pub fn with_pregel() -> Self {
        Self {
            graph: Graph::new(NodeTriggerMode::AnyPredecessor),
            _phantom: PhantomData,
        }
    }

    pub fn with_trigger_mode(mode: NodeTriggerMode) -> Self {
        Self {
            graph: Graph::new(mode),
            _phantom: PhantomData,
        }
    }

    pub fn add_node(
        mut self,
        key: impl Into<String>,
        runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>>,
    ) -> Self {
        self.graph.add_node(key, runnable);
        self
    }

    pub fn add_edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.graph.add_edge(from, to);
        self
    }

    pub fn compile(self) -> Result<CompiledGraph<I, serde_json::Value>, RunnableError> {
        self.graph.compile()
    }
}

impl<I, O> Default for Workflow<I, O> {
    fn default() -> Self {
        Self::new()
    }
}
