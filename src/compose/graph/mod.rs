pub mod channel;
pub mod options;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::runnable::{Runnable, RunnableFuture, RunnableResult, RunnableError, Context, StreamReader};

pub const START: &str = "start";
pub const END: &str = "end";

pub use options::{NodeTriggerMode, GraphCompileOptions, GraphCompileOption, WithMaxSteps, WithGraphName, WithTriggerMode};

#[derive(Clone)]
pub struct GraphNode {
    pub key: String,
    pub name: String,
    pub runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>>,
}

impl GraphNode {
    pub fn new(key: String, runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>>) -> Self {
        Self { name: runnable.name(), key, runnable }
    }
}

pub struct Graph<I, O> {
    nodes: HashMap<String, GraphNode>,
    control_edges: HashMap<String, Vec<String>>,
    data_edges: HashMap<String, Vec<String>>,
    branches: HashMap<String, Vec<GraphBranch>>,
    run_type: NodeTriggerMode,
    compiled: bool,
    _input: std::marker::PhantomData<I>,
    _output: std::marker::PhantomData<O>,
}

impl<I, O> Graph<I, O> {
    pub fn new(trigger_mode: NodeTriggerMode) -> Self {
        Self {
            nodes: HashMap::new(),
            control_edges: HashMap::new(),
            data_edges: HashMap::new(),
            branches: HashMap::new(),
            run_type: trigger_mode,
            compiled: false,
            _input: std::marker::PhantomData,
            _output: std::marker::PhantomData,
        }
    }

    pub fn add_node(&mut self, key: impl Into<String>, runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>>) -> &mut GraphNode {
        let key_str = key.into();
        match self.nodes.entry(key_str.clone()) {
            std::collections::hash_map::Entry::Vacant(e) => e.insert(GraphNode::new(key_str.clone(), runnable)),
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        };
        self.nodes.get_mut(&key_str).unwrap()
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        let from = from.into();
        let to = to.into();
        self.control_edges.entry(from.clone()).or_default().push(to.clone());
        self.data_edges.entry(from).or_default().push(to);
        self
    }

    pub fn add_lambda_node<F>(&mut self, name: &str, f: F) -> &mut Self
    where F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static {
        let lambda = SyncLambda { name: name.to_string(), f: Arc::new(f) };
        let runnable: Arc<dyn Runnable<serde_json::Value, serde_json::Value>> = Arc::new(lambda);
        self.add_node(name, runnable);
        self
    }

    pub fn add_conditional_edge<F>(&mut self, from: &str, condition: F, to: &str) -> &mut Self
    where F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static {
        let to_owned = to.to_string();
        let branch = GraphBranch::new(
            move |_ctx: &Context, input: &serde_json::Value| {
                Ok(if condition(input) { Some(to_owned.clone()) } else { None })
            },
            std::iter::once(to.to_string()).collect(),
        );
        self.add_branch(from, branch);
        self
    }

    pub fn add_branch(&mut self, from: impl Into<String>, branch: GraphBranch) -> &mut Self {
        self.branches.entry(from.into()).or_default().push(branch);
        self
    }

    pub fn compile(self) -> Result<CompiledGraph<I, O>, RunnableError> {
        self.compile_with_options(GraphCompileOptions::default())
    }

    pub fn compile_with_options(self, opts: GraphCompileOptions) -> Result<CompiledGraph<I, O>, RunnableError> {
        if self.compiled {
            return Err(RunnableError::GraphCompiled);
        }
        Ok(CompiledGraph {
            inner: Arc::new(CompiledGraphInner {
                nodes: self.nodes,
                control_edges: self.control_edges,
                data_edges: self.data_edges,
                branches: self.branches,
                run_type: self.run_type,
                options: opts,
            }),
            _input: std::marker::PhantomData,
            _output: std::marker::PhantomData,
        })
    }
}

impl<I, O> Default for Graph<I, O> {
    fn default() -> Self {
        Self::new(NodeTriggerMode::default())
    }
}

#[derive(Clone)]
struct CompiledGraphInner {
    nodes: HashMap<String, GraphNode>,
    control_edges: HashMap<String, Vec<String>>,
    data_edges: HashMap<String, Vec<String>>,
    branches: HashMap<String, Vec<GraphBranch>>,
    run_type: NodeTriggerMode,
    options: GraphCompileOptions,
}

pub struct CompiledGraph<I, O> {
    inner: Arc<CompiledGraphInner>,
    _input: std::marker::PhantomData<I>,
    _output: std::marker::PhantomData<O>,
}

impl<I: Send + 'static + serde::Serialize, O: Send + 'static + serde::de::DeserializeOwned> CompiledGraph<I, O> {
    pub async fn invoke(&self, ctx: Context, input: I) -> RunnableResult<O> {
        let input_json = serde_json::to_value(&input)
            .map_err(|e| RunnableError::InvalidInput(e.to_string()))?;

        let deps: HashMap<String, Vec<String>> = self.inner.control_edges.clone();
        let channel: Arc<dyn channel::Channel> = match self.inner.run_type {
            NodeTriggerMode::AllPredecessor => Arc::new(channel::DagChannel::with_dependencies(deps)),
            NodeTriggerMode::AnyPredecessor => Arc::new(channel::PregelChannel::with_dependencies(deps)),
        };

        channel.report(START, input_json)
            .map_err(|e| RunnableError::Invoke(e))?;

        execute_graph(&self.inner, &ctx, &channel).await?;

        let output = channel.get(END)
            .map_err(|e| RunnableError::Invoke(e))?
            .ok_or_else(|| RunnableError::Invoke("no output from graph".to_string()))?;

        serde_json::from_value(output)
            .map_err(|e| RunnableError::Invoke(e.to_string()))
    }
}

async fn execute_graph(
    inner: &CompiledGraphInner,
    ctx: &Context,
    channel: &Arc<dyn channel::Channel>,
) -> RunnableResult<()> {
    let max_steps = inner.options.max_steps;
    let mut step = 0;

    loop {
        step += 1;
        if step > max_steps {
            return Err(RunnableError::Invoke(format!("exceeded max steps {}", max_steps)));
        }

        let ready = find_ready_nodes(inner, channel);

        if ready.is_empty() && channel.done_nodes().contains(&END.to_string()) {
            break;
        }

        if ready.is_empty() {
            break;
        }

        let handles: Vec<_> = ready.into_iter().map(|key| {
            let inner = inner.clone();
            let ctx = ctx.clone();
            let channel = channel.clone();
            let key = key.clone();
            tokio::spawn(async move {
                execute_node(&inner, &ctx, &channel, &key).await
            })
        }).collect();

        for h in handles {
            h.await
                .map_err(|e| RunnableError::Invoke(e.to_string()))??;
        }
    }
    Ok(())
}

fn find_ready_nodes(inner: &CompiledGraphInner, channel: &Arc<dyn channel::Channel>) -> Vec<String> {
    let mut ready = Vec::new();

    for (key, _) in &inner.nodes {
        if channel.done_nodes().contains(key) {
            continue;
        }

        let deps: Vec<String> = inner.control_edges.iter()
        .filter(|(_, targets)| targets.iter().any(|k| k == key))
            .map(|(k, _)| k.clone())
            .collect();

        let is_ready = match inner.run_type {
            NodeTriggerMode::AllPredecessor => {
                deps.is_empty() || channel.all_predecessors_done(key, &deps)
            }
            NodeTriggerMode::AnyPredecessor => {
                deps.is_empty() || deps.iter().any(|d| channel.done_nodes().contains(d))
            }
        };

        if is_ready {
            ready.push(key.clone());
        }
    }

    ready
}

async fn execute_node(
    inner: &CompiledGraphInner,
    ctx: &Context,
    channel: &Arc<dyn channel::Channel>,
    key: &str,
) -> RunnableResult<()> {
    let node = inner.nodes.get(key)
        .ok_or_else(|| RunnableError::Invoke(format!("node {} not found", key)))?;

    let source_keys: Vec<String> = inner.data_edges.iter()
        .filter(|(_, targets)| targets.iter().any(|k| k == key))
        .map(|(k, _)| k.clone())
        .collect();
    let source = if source_keys.is_empty() { START.to_string() } else { source_keys[0].clone() };

    let input = channel.get(&source)
        .map_err(|e| RunnableError::Invoke(e))?
        .unwrap_or(serde_json::Value::Null);

    let output = node.runnable.invoke(ctx.clone(), input).await?;

    channel.report(key, serde_json::to_value(&output).map_err(|e| RunnableError::Invoke(e.to_string()))?)
        .map_err(|e| RunnableError::Invoke(e))?;
    channel.mark_done(key)
        .map_err(|e| RunnableError::Invoke(e))?;

    if let Some(branches) = inner.branches.get(key) {
        for branch in branches {
            if let Some(target) = (branch.condition)(ctx, &output)? {
                if let Some(edges) = inner.control_edges.get(key) {
                    for edge in edges {
                        if edge == &target {
                            continue;
                        }
                            channel.report(edge, serde_json::Value::Null)
                                .map_err(|e| RunnableError::Invoke(e))?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct GraphBranch {
    condition: Arc<dyn Fn(&Context, &serde_json::Value) -> RunnableResult<Option<String>> + Send + Sync>,
    end_nodes: HashSet<String>,
}

impl GraphBranch {
    pub fn new<C>(condition: C, end_nodes: HashSet<String>) -> Self
    where C: Fn(&Context, &serde_json::Value) -> RunnableResult<Option<String>> + Send + Sync + 'static {
        Self { condition: Arc::new(condition), end_nodes }
    }
}

struct SyncLambda<F> {
    name: String,
    f: Arc<F>,
}

impl<F> SyncLambda<F>
where F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
{
    fn invoke_sync(&self, input: serde_json::Value) -> RunnableResult<serde_json::Value> {
        (self.f)(input).map_err(|e| RunnableError::Invoke(e))
    }
}

impl<F> Runnable<serde_json::Value, serde_json::Value> for SyncLambda<F>
where F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
{
    fn invoke(&self, _ctx: Context, input: serde_json::Value) -> crate::compose::runnable::RunnableFuture<serde_json::Value> {
        let f = self.f.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                (f)(input).map_err(|e| RunnableError::Invoke(e))
            }).await.map_err(|e| RunnableError::Invoke(e.to_string()))?
        })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

pub fn new_graph<I, O>() -> Graph<I, O> {
    Graph::new(NodeTriggerMode::default())
}
