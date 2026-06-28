use std::sync::Arc;
use tokio::sync::Mutex;

use crate::components::tool::{BaseTool, InvokableTool, ToolError};
use crate::mcp::{McpTool, StdioMcpClient};
use crate::schema::ToolInfo;

pub struct McpToolAdapter {
    tool: McpTool,
    client: Arc<Mutex<StdioMcpClient>>,
}

impl McpToolAdapter {
    pub fn new(tool: McpTool, client: Arc<Mutex<StdioMcpClient>>) -> Self {
        Self { tool, client }
    }

    pub fn name(&self) -> &str {
        &self.tool.name
    }
}

impl BaseTool for McpToolAdapter {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        let name = self.tool.name.clone();
        let description = self.tool.description.clone().unwrap_or_default();
        let parameters = self
            .tool
            .input_schema
            .clone()
            .unwrap_or(serde_json::json!({}));
        async move {
            Ok(ToolInfo {
                name,
                description,
                parameters,
            })
        }
    }
}

#[async_trait::async_trait]
impl InvokableTool for McpToolAdapter {
    async fn invoke(&self, arguments: String) -> Result<String, ToolError> {
        let arguments: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(format!("parse arguments: {}", e)))?;

        let tool_name = self.tool.name.clone();
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut c = client.blocking_lock();
            c.call_tool(&tool_name, arguments)
        })
        .await
        .map_err(|e| ToolError::Execution(format!("blocking task: {}", e)))?
        .map_err(|e| ToolError::Execution(format!("mcp call: {}", e)))?;

        if result.is_error {
            return Err(ToolError::Execution(format!(
                "mcp tool error: {:?}",
                result.content
            )));
        }

        let mut out = String::new();
        for content in result.content {
            if let crate::mcp::McpContent::Text(t) = content {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&t.text);
            }
        }
        Ok(out)
    }
}
