pub mod channel;

use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;

pub const START: &str = "start";
pub const END: &str = "end";

pub struct Graph<I, O> {
    nodes: HashMap<String, GraphNode>,
    edges: HashMap<String, Vec<String>>,
    branches: HashMap<String, Vec<Branch>>,
    _input: std::marker::PhantomData<I>,
    _output: std::marker::PhantomData<O>,
}

unsafe impl<I, O> Send for Graph<I, O> {}
unsafe impl<I, O> Sync for Graph<I, O> {}

pub struct GraphNode {
    pub name: String,
    pub node_type: NodeType,
}

pub enum NodeType {
    Lambda(Arc<dyn Send + Sync + Fn(serde_json::Value) -> Result<serde_json::Value, String>>),
    AsyncLambda(Arc<dyn Send + Sync + Fn(serde_json::Value) -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> + 'static>),
    ChatModel,
    Tool,
    Retriever,
}

pub struct Branch {
    pub condition: Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>,
    pub target: String,
}

impl<I, O> Graph<I, O> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            branches: HashMap::new(),
            _input: std::marker::PhantomData,
            _output: std::marker::PhantomData,
        }
    }
    
    pub fn add_node(&mut self, name: &str, node_type: NodeType) -> &mut Self {
        self.nodes.insert(
            name.to_string(),
            GraphNode {
                name: name.to_string(),
                node_type,
            },
        );
        self
    }
    
    pub fn add_lambda_node<F>(&mut self, name: &str, f: F) -> &mut Self
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
    {
        self.add_node(name, NodeType::Lambda(Arc::new(f)))
    }
    
    pub fn add_async_lambda_node<F, Fut>(&mut self, name: &str, f: F) -> &mut Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        let boxed = Arc::new(move |input: serde_json::Value| -> Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>> {
            Box::pin(f(input))
        }) as _;
        self.add_node(name, NodeType::AsyncLambda(boxed))
    }
    
    pub fn add_edge(&mut self, from: &str, to: &str) -> &mut Self {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
        self
    }
    
    pub fn add_conditional_edge<F>(&mut self, from: &str, condition: F, to: &str) -> &mut Self
    where
        F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
    {
        self.branches
            .entry(from.to_string())
            .or_default()
            .push(Branch {
                condition: Arc::new(condition),
                target: to.to_string(),
            });
        self
    }
    
    pub fn compile(self) -> Result<CompiledGraph<I, O>, String> {
        if self.nodes.is_empty() {
            return Err("Graph has no nodes".to_string());
        }
        Ok(CompiledGraph {
            graph: Arc::new(self),
        })
    }
    
    pub async fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, String> {
        let mut current = input;
        let mut visited = HashMap::new();
        let mut current_node = START.to_string();
        
        loop {
            if current_node == END {
                return Ok(current);
            }
            
            let visit_count = visited.entry(current_node.clone()).or_insert(0);
            *visit_count += 1;
            if *visit_count > 100 {
                return Err("Graph execution exceeded max iterations".to_string());
            }
            
            let node = self.nodes.get(&current_node)
                .ok_or_else(|| format!("Node '{}' not found", current_node))?;
            
            match &node.node_type {
                NodeType::Lambda(f) => {
                    current = f(current)?;
                }
                NodeType::AsyncLambda(f) => {
                    current = f(current).await?;
                }
                NodeType::ChatModel => {
                    return Err("ChatModel node - use add_async_lambda_node with async function".to_string());
                }
                NodeType::Tool => {
                    return Err("Tool node - use add_async_lambda_node with async function".to_string());
                }
                NodeType::Retriever => {
                    return Err("Retriever node - use add_async_lambda_node with async function".to_string());
                }
            }
            
            let next = self.find_next_node(&current_node, &current);
            current_node = next;
        }
    }
    
    fn find_next_node(&self, current: &str, value: &serde_json::Value) -> String {
        if let Some(branches) = self.branches.get(current) {
            for branch in branches {
                if (branch.condition)(value) {
                    return branch.target.clone();
                }
            }
        }
        
        if let Some(edges) = self.edges.get(current) {
            if let Some(first) = edges.first() {
                return first.clone();
            }
        }
        
        END.to_string()
    }
}

impl<I, O> Default for Graph<I, O> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CompiledGraph<I, O> {
    graph: Arc<Graph<I, O>>,
}

impl<I: Send + 'static + serde::Serialize, O: Send + 'static + for<'de> serde::Deserialize<'de>> CompiledGraph<I, O> {
    pub async fn invoke(&self, input: I) -> Result<O, String> {
        let json_input = serde_json::to_value(input)
            .map_err(|e| format!("Failed to serialize input: {}", e))?;
        
        let result = self.graph.execute(json_input).await?;
        
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to deserialize output: {}", e))
    }
    
    pub async fn stream(&self, input: I) -> Result<crate::components::model::BoxStreamReader, String> {
        let json_input = serde_json::to_value(input)
            .map_err(|e| format!("Failed to serialize input: {}", e))?;
        
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let graph = self.graph.clone();
        
        tokio::spawn(async move {
            let mut current = json_input;
            let mut visited = HashMap::new();
            let mut current_node = START.to_string();
            
            loop {
                if current_node == END {
                    let _ = tx.send(Ok(crate::schema::Message {
                        role: crate::schema::RoleType::Assistant,
                        content: crate::schema::Content::Text("Graph execution completed".to_string()),
                        name: None,
                        tool_call_id: None,
                        tool_calls: None,
                    })).await;
                    break;
                }
                
                let visit_count = visited.entry(current_node.clone()).or_insert(0);
                *visit_count += 1;
                if *visit_count > 100 {
                    let _ = tx.send(Err(crate::components::model::ModelError::StreamError(
                        "Graph execution exceeded max iterations".to_string()
                    ))).await;
                    break;
                }
                
                let node = match graph.nodes.get(&current_node) {
                    Some(n) => n,
                    None => {
                        let _ = tx.send(Err(crate::components::model::ModelError::StreamError(
                            format!("Node '{}' not found", current_node)
                        ))).await;
                        break;
                    }
                };
                
                match &node.node_type {
                    NodeType::Lambda(f) => {
                        match f(current.clone()) {
                            Ok(result) => {
                                current = result;
                                let _ = tx.send(Ok(crate::schema::Message {
                                    role: crate::schema::RoleType::Assistant,
                                    content: crate::schema::Content::Text(serde_json::to_string(&current).unwrap_or_default()),
                                    name: None,
                                    tool_call_id: None,
                                    tool_calls: None,
                                })).await;
                            }
                            Err(e) => {
                                let _ = tx.send(Err(crate::components::model::ModelError::StreamError(e))).await;
                                break;
                            }
                        }
                    }
                    NodeType::AsyncLambda(f) => {
                        match f(current.clone()).await {
                            Ok(result) => {
                                current = result;
                                let _ = tx.send(Ok(crate::schema::Message {
                                    role: crate::schema::RoleType::Assistant,
                                    content: crate::schema::Content::Text(serde_json::to_string(&current).unwrap_or_default()),
                                    name: None,
                                    tool_call_id: None,
                                    tool_calls: None,
                                })).await;
                            }
                            Err(e) => {
                                let _ = tx.send(Err(crate::components::model::ModelError::StreamError(e))).await;
                                break;
                            }
                        }
                    }
                    NodeType::ChatModel => {
                        let _ = tx.send(Err(crate::components::model::ModelError::StreamError(
                            "ChatModel node - use add_async_lambda_node".to_string()
                        ))).await;
                        break;
                    }
                    NodeType::Tool => {
                        let _ = tx.send(Err(crate::components::model::ModelError::StreamError(
                            "Tool node - use add_async_lambda_node".to_string()
                        ))).await;
                        break;
                    }
                    NodeType::Retriever => {
                        let _ = tx.send(Err(crate::components::model::ModelError::StreamError(
                            "Retriever node - use add_async_lambda_node".to_string()
                        ))).await;
                        break;
                    }
                }
                
                current_node = graph.find_next_node(&current_node, &current);
            }
        });
        
        let stream_reader = crate::components::model::StreamReader::from_receiver(rx);
        Ok(Box::pin(stream_reader))
    }
}

pub fn new_graph<I, O>() -> Graph<I, O> {
    Graph::new()
}
