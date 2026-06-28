# Tool

Tools extend an LLM agent's capabilities. The LLM can decide to "call" a
tool by emitting a `tool_call` in its response; the agent runtime then
executes the tool and feeds the result back to the LLM.

## Trait Hierarchy

```rust
pub trait BaseTool: Send + Sync {
    fn info(&self) -> impl Future<Output = Result<ToolInfo, ToolError>> + Send;
}

#[async_trait]
pub trait InvokableTool: BaseTool + Send + Sync {
    async fn invoke(&self, arguments: String) -> Result<String, ToolError>;
}

#[async_trait]
pub trait StreamableTool: BaseTool + Send + Sync {
    async fn stream(&self, arguments: String) -> Result<StringStreamReader, ToolError>;
}

#[async_trait]
pub trait EnhancedInvokableTool: BaseTool + Send + Sync {
    async fn invoke_enhanced(&self, arguments: ToolArgument)
        -> Result<ToolResult, ToolError>;
}
```

## ToolInfo Schema

A tool's `info()` returns a `ToolInfo` describing its name, description,
and JSON Schema for arguments:

```rust
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}
```

This is sent to the LLM so it knows what tools are available.

## Built-in Tool Sources

| Source | Description |
|--------|-------------|
| Custom `InvokableTool` | User-defined tools (Calculator, Search, etc.) |
| `McpToolAdapter` | Wraps MCP tools (stdio or HTTP transport) |
| `AgentTool` | Wraps any `Agent` as a tool (for MultiAgent) |

## Tool Middleware

`ChatModelAgent` supports pre/post tool execution hooks via
`ToolMiddleware`:

```rust
pub trait ToolMiddleware: Send + Sync {
    fn before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError>;
    fn after_tool(&self, name: &str, result: &str) -> Result<(), ToolError>;
}

let agent = ChatModelAgent::builder()
    .with_tool_middleware(Arc::new(LoggingToolMiddleware::new()))
    .build();
```

## Example: HTTP Tool

```rust
use async_trait::async_trait;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use synapse::schema::ToolInfo;
use synapse::transport::{DefaultHttpClient, HttpClient, HttpRequest, HttpMethod};

struct HttpGetTool {
    client: Arc<dyn HttpClient>,
}

#[async_trait]
impl BaseTool for HttpGetTool {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move {
            Ok(ToolInfo {
                name: "http_get".into(),
                description: "Performs HTTP GET request".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "URL to fetch"}
                    },
                    "required": ["url"]
                }),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for HttpGetTool {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;
        let url = args["url"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("missing url".into()))?;

        let response = self.client.execute(HttpRequest {
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: vec![],
            body: None,
        }).await.map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(String::from_utf8_lossy(&response.body).to_string())
    }
}
```

This pattern is used internally by `McpToolAdapter`, `A2ATool`, etc.
