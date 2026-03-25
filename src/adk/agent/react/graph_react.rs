use std::sync::Arc;
use async_trait::async_trait;
use crate::schema::{Message, RoleType, Content};
use crate::components::model::{BaseChatModel, GenerateOptions};
use crate::compose::{Graph, CompiledGraph, START, END, NodeTriggerMode};
use crate::graph::{GraphBranch, GraphCompileOptions};
use crate::compose::runnable::{Runnable, RunnableFuture, RunnableError, Context};
use crate::adk::agent::{Agent, AgentInput, AgentEvent, AgentError, AgentOutput, AgentResult};

pub struct GraphReActAgent {
    name: String,
    description: String,
    compiled_graph: Arc<CompiledGraph<Vec<Message>, Message>>,
    max_steps: usize,
}

impl GraphReActAgent {
    pub fn builder() -> GraphReActAgentBuilder {
        GraphReActAgentBuilder::new()
    }
}

#[async_trait]
impl Agent for GraphReActAgent {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let compiled_graph = self.compiled_graph.clone();
        
        let output = compiled_graph
            .invoke(Context::new(), input.messages)
            .await
            .map_err(|e| AgentError::Agent(e.to_string()))?;
            
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(output),
                action: None,
            },
            error: None,
        })
    }

    async fn resume(&self, _checkpoint: Vec<u8>, _info: crate::adk::agent::ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct GraphReActAgentBuilder {
    name: String,
    description: String,
    model: Option<Arc<dyn BaseChatModel>>,
    max_steps: usize,
}

impl GraphReActAgentBuilder {
    pub fn new() -> Self {
        Self {
            name: "ReActAgent".to_string(),
            description: "".to_string(),
            model: None,
            max_steps: 12,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_model(mut self, model: Arc<dyn BaseChatModel>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    pub fn build(self) -> Result<GraphReActAgent, AgentError> {
        let model = self.model.ok_or_else(|| AgentError::InvalidInput("model is required".to_string()))?;
        
        let compiled_graph = build_graph(model, self.max_steps)?;
        
        Ok(GraphReActAgent {
            name: self.name,
            description: self.description,
            compiled_graph: Arc::new(compiled_graph),
            max_steps: self.max_steps,
        })
    }
}

struct ChatModelNode {
    model: Arc<dyn BaseChatModel>,
}

impl ChatModelNode {
    fn new(model: Arc<dyn BaseChatModel>) -> Self {
        Self { model }
    }
}

impl Runnable<serde_json::Value, serde_json::Value> for ChatModelNode {
    fn invoke(&self, ctx: Context, input: serde_json::Value) -> RunnableFuture<serde_json::Value> {
        let model = self.model.clone();
        
        Box::pin(async move {
            let messages: Vec<Message> = serde_json::from_value(input)
                .map_err(|e| RunnableError::InvalidInput(e.to_string()))?;
            
            let options = GenerateOptions::default();
            
            let output = model.generate(messages, options).await
                .map_err(|e| RunnableError::Invoke(e.to_string()))?;
            
            serde_json::to_value(output)
                .map_err(|e| RunnableError::Invoke(e.to_string()))
        })
    }

    fn name(&self) -> String {
        "chat_model".to_string()
    }
}

fn build_graph(
    model: Arc<dyn BaseChatModel>,
    max_steps: usize,
) -> Result<CompiledGraph<Vec<Message>, Message>, AgentError> {
    let mut graph = Graph::<Vec<Message>, Message>::new(NodeTriggerMode::AnyPredecessor);
    
    let chat_node = ChatModelNode::new(model);
    graph.add_node("chat", Arc::new(chat_node));
    
    let tools_node = ToolsGraphNode::new();
    graph.add_node("tools", Arc::new(tools_node));
    
    graph.add_edge(START, "chat");
    
    let has_tool_calls_branch = GraphBranch::new(
        move |_ctx: &Context, input: &serde_json::Value| {
            if let Ok(msg) = serde_json::from_value::<Message>(input.clone()) {
                if msg.tool_calls.is_some() && !msg.tool_calls.as_ref().unwrap().is_empty() {
                    return Ok(Some("tools".to_string()));
                }
            }
            Ok(None)
        },
        vec!["chat".to_string(), END.to_string()].into_iter().collect(),
    );
    graph.add_branch("chat", has_tool_calls_branch);
    
    graph.add_edge("tools", "chat");
    
    let return_directly_branch = GraphBranch::new(
        move |_ctx: &Context, _input: &serde_json::Value| {
            Ok(None)
        },
        vec!["tools".to_string(), END.to_string()].into_iter().collect(),
    );
    graph.add_branch("tools", return_directly_branch);
    
    graph.compile_with_options(GraphCompileOptions {
        max_steps,
        ..Default::default()
    }).map_err(|e| AgentError::Agent(e.to_string()))
}

struct ToolsGraphNode;

impl ToolsGraphNode {
    fn new() -> Self {
        Self
    }
}

impl Runnable<serde_json::Value, serde_json::Value> for ToolsGraphNode {
    fn invoke(&self, _ctx: Context, input: serde_json::Value) -> RunnableFuture<serde_json::Value> {
        Box::pin(async move {
            let msg: Message = serde_json::from_value(input)
                .map_err(|e| RunnableError::InvalidInput(e.to_string()))?;
            
            let tool_calls = msg.tool_calls.clone().unwrap_or_default();
            if tool_calls.is_empty() {
                return serde_json::to_value(&msg)
                    .map_err(|e| RunnableError::Invoke(e.to_string()));
            }
            
            let mut result_messages = Vec::new();
            for tc in tool_calls {
                let tool_result_msg = Message {
                    role: RoleType::Tool,
                    content: Content::Text(format!("Tool {} result: tool not found", tc.function.name)),
                    name: Some(tc.function.name),
                    tool_call_id: Some(tc.id),
                    tool_calls: None,
                };
                result_messages.push(tool_result_msg);
            }
            
            serde_json::to_value(result_messages)
                .map_err(|e| RunnableError::Invoke(e.to_string()))
        })
    }

    fn name(&self) -> String {
        "tools".to_string()
    }
}
