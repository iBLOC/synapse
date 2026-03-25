use std::sync::Arc;
use async_trait::async_trait;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError};
use synapse::schema::{Message, RoleType, ToolCall, FunctionCall, ToolInfo};
use synapse::compose::tool_node::{ToolsNode, ToolsNodeConfig};
use synapse::compose::runnable::Runnable;
use synapse::compose::runnable::Context;

struct TestTool {
    name: String,
    result: String,
}

impl TestTool {
    fn new(name: &str, result: &str) -> Arc<Self> {
        Arc::new(Self { name: name.to_string(), result: result.to_string() })
    }
}

impl BaseTool for TestTool {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move {
            Ok(ToolInfo {
                name: self.name.clone(),
                description: "test tool".to_string(),
                parameters: serde_json::json!({}),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for TestTool {
    async fn invoke(&self, _arguments: String) -> Result<String, ToolError> {
        Ok(self.result.clone())
    }
}

#[tokio::test]
async fn test_tools_node_single_tool() {
    let tool = TestTool::new("get_weather", "sunny day");
    let config = ToolsNodeConfig::<TestTool> {
        tools: vec![tool],
        ..Default::default()
    };
    let node = ToolsNode::new(config).unwrap();

    let input = Message {
        role: RoleType::Assistant,
        content: synapse::schema::Content::Text("what is weather".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![ToolCall {
            id: "call_1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "get_weather".to_string(),
                arguments: "{}".to_string(),
            },
        }]),
    };

    let ctx = Context::new();
    let output = node.invoke(ctx, input).await.unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(output[0].name.as_deref(), Some("get_weather"));
}

#[tokio::test]
async fn test_tools_node_multiple_tools_parallel() {
    let tool1 = TestTool::new("tool_a", "result_a");
    let tool2 = TestTool::new("tool_b", "result_b");
    let config = ToolsNodeConfig::<TestTool> {
        tools: vec![tool1, tool2],
        ..Default::default()
    };
    let node = ToolsNode::new(config).unwrap();

    let input = Message {
        role: RoleType::Assistant,
        content: synapse::schema::Content::Text("run both".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![
            ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "tool_a".to_string(),
                    arguments: "{}".to_string(),
                },
            },
            ToolCall {
                id: "call_2".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "tool_b".to_string(),
                    arguments: "{}".to_string(),
                },
            },
        ]),
    };

    let ctx = Context::new();
    let output = node.invoke(ctx, input).await.unwrap();
    assert_eq!(output.len(), 2);
}

#[tokio::test]
async fn test_tools_node_unknown_tool_handler() {
    let handler = Arc::new(|name: String, _input: String| Ok(format!("unknown: {}", name)));
    let config = ToolsNodeConfig::<TestTool> {
        tools: vec![],
        unknown_tool_handler: Some(handler),
        ..Default::default()
    };
    let node = ToolsNode::new(config).unwrap();

    let input = Message {
        role: RoleType::Assistant,
        content: synapse::schema::Content::Text("call unknown".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![ToolCall {
            id: "call_x".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "non_existent".to_string(),
                arguments: "{}".to_string(),
            },
        }]),
    };

    let ctx = Context::new();
    let output = node.invoke(ctx, input).await.unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].tool_call_id.as_deref(), Some("call_x"));
    if let synapse::schema::Content::Text(t) = &output[0].content {
        assert_eq!(t, "unknown: non_existent");
    } else {
        panic!("expected text content");
    }
}
