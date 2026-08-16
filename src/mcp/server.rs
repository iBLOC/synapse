use async_trait::async_trait;
use serde_json::Value;

use super::types::{McpCallToolResult, McpTool};
use super::McpResult;

#[async_trait]
pub trait McpServer: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    async fn list_tools(&self) -> McpResult<Vec<McpTool>>;
    async fn call_tool(&self, name: &str, arguments: Value) -> McpResult<McpCallToolResult>;
}

pub struct SimpleMcpServer {
    name: String,
    version: String,
    tools: Vec<McpTool>,
    handlers: std::collections::HashMap<String, Box<dyn ToolHandler>>,
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, arguments: Value) -> McpResult<McpCallToolResult>;
}

impl SimpleMcpServer {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: Vec::new(),
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn with_tool(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Option<Value>,
        handler: impl ToolHandler + 'static,
    ) -> Self {
        let name_str = name.into();
        self.tools.push(McpTool {
            name: name_str.clone(),
            description: Some(description.into()),
            input_schema,
        });
        self.handlers.insert(name_str, Box::new(handler));
        self
    }
}

#[async_trait]
impl McpServer for SimpleMcpServer {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn list_tools(&self) -> McpResult<Vec<McpTool>> {
        Ok(self.tools.clone())
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> McpResult<McpCallToolResult> {
        let handler = self.handlers.get(name)
            .ok_or_else(|| super::McpError::Rpc {
                code: -32601,
                message: format!("tool not found: {name}"),
            })?;
        handler.execute(arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::McpTextContent;

    struct EchoHandler;

    #[async_trait]
    impl ToolHandler for EchoHandler {
        async fn execute(&self, arguments: Value) -> McpResult<McpCallToolResult> {
            let text = arguments.get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("no input");
            Ok(McpCallToolResult {
                content: vec![super::super::McpContent::Text(McpTextContent {
                    text: text.to_string(),
                })],
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn test_simple_server_list_tools() {
        let server = SimpleMcpServer::new("test", "1.0")
            .with_tool("echo", "Echoes input", None, EchoHandler);

        let tools = server.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[tokio::test]
    async fn test_simple_server_call_tool() {
        let server = SimpleMcpServer::new("test", "1.0")
            .with_tool("echo", "Echoes input", None, EchoHandler);

        let result = server.call_tool("echo", serde_json::json!({"input": "hello"})).await.unwrap();
        match &result.content[0] {
            super::super::McpContent::Text(t) => assert_eq!(t.text, "hello"),
        }
    }

    #[tokio::test]
    async fn test_simple_server_call_unknown_tool() {
        let server = SimpleMcpServer::new("test", "1.0");
        let result = server.call_tool("unknown", serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
