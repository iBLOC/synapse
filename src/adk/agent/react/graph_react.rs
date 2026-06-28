use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, AgentResult};
use crate::components::model::{BaseChatModel, GenerateOptions};
use crate::compose::runnable::{Context, Runnable, RunnableError, RunnableFuture};
use crate::compose::tool_node::ToolsNode;
use crate::compose::{CompiledGraph, Graph, NodeTriggerMode, END, START};
use crate::graph::GraphCompileOptions;
use crate::schema::Message;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GraphReActAgent {
    name: String,
    description: String,
    tools_node: Option<Arc<ToolsNode>>,
    max_steps: usize,
    compiled_graph: Arc<CompiledGraph<Vec<Message>, Message>>,
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
        let mut history = input.messages;

        for _ in 0..self.max_steps {
            let response = self
                .compiled_graph
                .invoke(Context::new(), history.clone())
                .await
                .map_err(|e| AgentError::Agent(e.to_string()))?;

            let tool_calls = response.tool_calls.clone().unwrap_or_default();
            if tool_calls.is_empty() {
                return Ok(AgentEvent {
                    agent_name: self.name.clone(),
                    output: AgentOutput {
                        message: Some(response),
                        action: None,
                    },
                    error: None,
                });
            }

            history.push(response.clone());

            let tools_node = self.tools_node.as_ref().ok_or_else(|| {
                AgentError::Agent("tool_calls present but no tools configured".to_string())
            })?;

            let tool_results: Vec<Message> = tools_node
                .invoke(Context::new(), response)
                .await
                .map_err(|e| AgentError::Agent(e.to_string()))?;

            history.extend(tool_results);
        }

        Err(AgentError::MaxIterationsExceeded)
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: crate::adk::agent::ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct GraphReActAgentBuilder {
    name: String,
    description: String,
    model: Option<Arc<dyn BaseChatModel>>,
    tools_node: Option<Arc<ToolsNode>>,
    max_steps: usize,
}

impl GraphReActAgentBuilder {
    pub fn new() -> Self {
        Self {
            name: "ReActAgent".to_string(),
            description: "".to_string(),
            model: None,
            tools_node: None,
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

    pub fn with_tools_node(mut self, tools_node: Arc<ToolsNode>) -> Self {
        self.tools_node = Some(tools_node);
        self
    }

    pub fn max_steps(mut self, max: usize) -> Self {
        self.max_steps = max;
        self
    }

    pub fn build(self) -> Result<GraphReActAgent, AgentError> {
        let model = self
            .model
            .ok_or_else(|| AgentError::InvalidInput("model is required".to_string()))?;

        let compiled_graph = build_graph(model, self.max_steps)?;

        Ok(GraphReActAgent {
            name: self.name,
            description: self.description,
            tools_node: self.tools_node,
            max_steps: self.max_steps,
            compiled_graph: Arc::new(compiled_graph),
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
    fn invoke(&self, _ctx: Context, input: serde_json::Value) -> RunnableFuture<serde_json::Value> {
        let model = self.model.clone();

        Box::pin(async move {
            let messages: Vec<Message> = serde_json::from_value(input)
                .map_err(|e| RunnableError::InvalidInput(e.to_string()))?;

            let options = GenerateOptions::default();

            let output = model
                .generate(messages, options)
                .await
                .map_err(|e| RunnableError::Invoke(e.to_string()))?;

            serde_json::to_value(output).map_err(|e| RunnableError::Invoke(e.to_string()))
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

    graph.add_edge(START, "chat");
    graph.add_edge("chat", END);

    graph
        .compile_with_options(GraphCompileOptions {
            max_steps: max_steps * 4 + 1,
            ..Default::default()
        })
        .map_err(|e| AgentError::Agent(e.to_string()))
}
