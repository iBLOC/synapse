use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use synapse::adk::agent::react::GraphReActAgent;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use synapse::components::tool::{BaseTool, InvokableTool, ToolError};
use synapse::compose::tool_node::{ToolsNode, ToolsNodeConfig};
use synapse::schema::{Content, Message, RoleType, ToolInfo};

struct MockChatModel {
    responses: Vec<Message>,
    index: AtomicUsize,
}

impl MockChatModel {
    fn new(responses: Vec<Message>) -> Arc<Self> {
        Arc::new(Self {
            responses,
            index: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl BaseChatModel for MockChatModel {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<Message> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        if self.responses.is_empty() {
            return Err(ModelError::Api("No responses configured".to_string()));
        }
        let i = idx % self.responses.len();
        Ok(self.responses[i].clone())
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<synapse::components::model::StreamReader> {
        Err(ModelError::Api("Streaming not implemented".to_string()))
    }
}

#[tokio::test]
async fn test_graph_react_agent_builder() {
    let model = MockChatModel::new(vec![Message::assistant("Hello! How can I help you?")]);

    let agent = GraphReActAgent::builder()
        .name("TestAgent")
        .description("A test agent")
        .with_model(model)
        .max_steps(5)
        .build();

    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_graph_react_agent_name_and_description() {
    let model = MockChatModel::new(vec![Message::assistant("Hi")]);

    let agent = GraphReActAgent::builder()
        .name("MyAgent")
        .description("My test agent description")
        .with_model(model)
        .build()
        .unwrap();

    assert_eq!(agent.name(), "MyAgent");
    assert_eq!(agent.description(), "My test agent description");
}

#[tokio::test]
async fn test_graph_react_agent_run_no_tools() {
    let model = MockChatModel::new(vec![Message::assistant("Hello back!")]);

    let agent = GraphReActAgent::builder()
        .with_model(model)
        .max_steps(5)
        .build()
        .unwrap();

    let input = AgentInput {
        messages: vec![Message::user("Hi")],
        enable_streaming: false,
    };

    let result = agent.run(input).await;
    match result {
        Ok(event) => assert!(event.output.message.is_some()),
        Err(e) => panic!("agent.run failed: {}", e),
    }
}

struct TestTool {
    name: String,
    response: String,
}

impl TestTool {
    fn new(name: &str, response: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            response: response.to_string(),
        })
    }
}

impl BaseTool for TestTool {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        let name = self.name.clone();
        async move {
            Ok(ToolInfo {
                name,
                description: "test tool".to_string(),
                parameters: serde_json::json!({}),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for TestTool {
    async fn invoke(&self, _arguments: String) -> Result<String, ToolError> {
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn test_graph_react_agent_run_with_tool_call() {
    use synapse::compose::tool_node::ToolsNodeConfig;
    use synapse::schema::{FunctionCall, ToolCall};

    let model = MockChatModel::new(vec![
        Message {
            role: RoleType::Assistant,
            content: Content::Text("".to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "my_tool".to_string(),
                    arguments: "{}".to_string(),
                },
            }]),
        },
        Message::assistant("Final answer after tool call."),
    ]);

    let tool = TestTool::new("my_tool", "tool result text");
    let tools_node = Arc::new(
        ToolsNode::new(ToolsNodeConfig::<TestTool> {
            tools: vec![tool],
            ..Default::default()
        })
        .unwrap(),
    );

    let agent = GraphReActAgent::builder()
        .with_model(model)
        .with_tools_node(tools_node)
        .max_steps(5)
        .build()
        .unwrap();

    let input = AgentInput {
        messages: vec![Message::user("Use a tool")],
        enable_streaming: false,
    };

    let result = agent.run(input).await;
    match result {
        Ok(event) => {
            let msg = event.output.message.expect("expected a message");
            if let Content::Text(s) = msg.content {
                assert!(s.contains("Final answer"), "got: {}", s);
            } else {
                panic!("expected Text content");
            }
        }
        Err(e) => panic!("agent.run failed: {}", e),
    }
}

#[tokio::test]
async fn test_graph_react_agent_max_iterations() {
    use synapse::compose::tool_node::ToolsNodeConfig;
    use synapse::schema::{FunctionCall, ToolCall};

    let model = MockChatModel::new(vec![Message {
        role: RoleType::Assistant,
        content: Content::Text("".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![ToolCall {
            id: "call_1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "my_tool".to_string(),
                arguments: "{}".to_string(),
            },
        }]),
    }]);

    let tool = TestTool::new("my_tool", "always calls tool, never returns final");
    let tools_node = Arc::new(
        ToolsNode::new(ToolsNodeConfig::<TestTool> {
            tools: vec![tool],
            ..Default::default()
        })
        .unwrap(),
    );

    let agent = GraphReActAgent::builder()
        .with_model(model)
        .with_tools_node(tools_node)
        .max_steps(2)
        .build()
        .unwrap();

    let input = AgentInput {
        messages: vec![Message::user("Loop forever")],
        enable_streaming: false,
    };

    let result = agent.run(input).await;
    assert!(matches!(
        result,
        Err(synapse::adk::agent::AgentError::MaxIterationsExceeded)
    ));
}
