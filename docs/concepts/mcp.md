---
title: MCP (Model Context Protocol)
---

# MCP (Model Context Protocol)

MCP is a standard protocol for connecting LLMs to tools and data sources.
Synapse implements both MCP client and server.

## What is MCP?

MCP defines a JSON-RPC based protocol where:

- **Server** exposes tools, resources, prompts
- **Client** invokes them, gets results

## Transports

| Transport | Use Case | Class |
|-----------|----------|-------|
| **stdio** | Local subprocess (Python, Node, etc.) | `StdioMcpClient` |
| **HTTP** | Remote MCP server | `HttpMcpClient` |

Both implement the same `initialize / list_tools / call_tool` API.

## MCP Client

### StdioMcpClient

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn("python3", &["my_mcp_server.py"])?;
let init = client.initialize().await?;
let tools = client.list_tools().await?;
let result = client.call_tool("echo", serde_json::json!({"text": "hi"})).await?;
```

### HttpMcpClient

```rust
use std::sync::Arc;
use synapse::mcp::HttpMcpClient;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> = Arc::new(DefaultHttpClient::new());
let mut client = HttpMcpClient::new("https://mcp.example.com", http);
client.initialize().await?;
let tools = client.list_tools().await?;
```

## MCP Server

Synapse provides a trait-based MCP server abstraction:

### McpServer Trait

```rust
use synapse::mcp::McpServer;

#[async_trait]
impl McpServer for MyServer {
    fn name(&self) -> &str { "my-server" }
    fn version(&self) -> &str { "1.0" }

    async fn list_tools(&self) -> McpResult<Vec<McpTool>> { ... }
    async fn call_tool(&self, name: &str, args: Value) -> McpResult<McpCallToolResult> { ... }
}
```

### SimpleMcpServer

Build an MCP server with handler functions:

```rust
use synapse::mcp::{SimpleMcpServer, ToolHandler};

struct EchoHandler;

#[async_trait]
impl ToolHandler for EchoHandler {
    async fn execute(&self, args: Value) -> McpResult<McpCallToolResult> {
        let text = args["input"].as_str().unwrap_or("");
        Ok(McpCallToolResult {
            content: vec![McpContent::Text(McpTextContent { text: text.to_string() })],
            is_error: false,
        })
    }
}

let server = SimpleMcpServer::new("echo-server", "1.0")
    .with_tool("echo", "Echoes input back", None, EchoHandler);

let tools = server.list_tools().await?;
let result = server.call_tool("echo", serde_json::json!({"input": "hello"})).await?;
```

## Using MCP Tools in Synapse Agents

Wrap MCP tools as Synapse `InvokableTool` with `McpToolAdapter`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use synapse::mcp::tool::McpToolAdapter;
use synapse::mcp::McpTool;

let client = Arc::new(Mutex::new(client));
let mcp_tools: Vec<Arc<McpToolAdapter>> = tools.into_iter()
    .map(|t: McpTool| Arc::new(McpToolAdapter::new(t, client.clone())))
    .collect();
```

## See Also

- [MCP Server how-to](/howto/mcp-server)
