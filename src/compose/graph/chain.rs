use super::super::runnable::{invokable_lambda, Context, Runnable, RunnableError, RunnableResult};
use super::options::NodeTriggerMode;
use super::{CompiledGraph, Graph, END, START};
use std::marker::PhantomData;
use std::sync::Arc;

pub struct Chain<I> {
    graph: Graph<I, serde_json::Value>,
    last_key: String,
    node_count: usize,
}

impl<I> Chain<I> {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(NodeTriggerMode::AllPredecessor),
            last_key: START.to_string(),
            node_count: 0,
        }
    }

    pub fn append(
        mut self,
        runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>>,
    ) -> Self {
        self.node_count += 1;
        let key = format!("chain_node_{}", self.node_count);
        self.graph.add_node(key.clone(), runnable);
        self.graph.add_edge(&self.last_key, &key);
        self.last_key = key;
        self
    }

    pub fn append_lambda<F, Fut>(self, name: &str, f: F) -> Self
    where
        F: Fn(Context, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = RunnableResult<serde_json::Value>> + Send + 'static,
    {
        self.append(invokable_lambda(name, f))
    }

    pub fn compile(mut self) -> Result<CompiledGraph<I, serde_json::Value>, RunnableError> {
        self.graph.add_edge(&self.last_key, END);
        self.graph.compile()
    }
}

impl<I> Default for Chain<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I> std::fmt::Debug for Chain<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chain")
            .field("last_key", &self.last_key)
            .field("node_count", &self.node_count)
            .finish()
    }
}
