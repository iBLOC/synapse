# MCP (Model Context Protocol)

MCP is a standard protocol for connecting LLMs to tools and data sources.
Synapse implements MCP client (stdio + HTTP transports).

## What is MCP?

MCP defines a JSON-RPC based protocol where:
- **Server** exposes tools, resources, prompts
- **Client** invokes them, gets results

Synapse plays the **client** role, calling MCP servers and exposing
their tools to Synapse agents.

## Transports

| Transport | Use Case | Class |
|-----------|----------|-------|
| **stdio** | Local subprocess (Python, Node, etc.) | `StdioMcpClient` |
| **HTTP** | Remote MCP server | `HttpMcpClient` |

Both implement the same `initialize / list_tools / call_tool` API.

## StdioMcpClient

Spawns a subprocess and communicates via stdin/stdout using
Content-Length framed JSON-RPC (LSP-style).

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn(
    "python3",
    &["my_mcp_server.py"],
)?;

let init = client.initialize().await?;
let tools = client.list_tools().await?;
let result = client.call_tool("echo", serde_json::json!({"text": "hi"})).await?;
```

Or, for testing, use a pre-spawned child:

```rust
use std::process::Command;

let child = Command::new("python3")
    .args(&["mock_server.py"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

let mut client = StdioMcpClient::from_child(child)?;
```

## HttpMcpClient

Sends JSON-RPC over HTTP POST. Reuses the `HttpClient` trait, so it
works with both native (reqwest) and WASM (wasi::http) transports.

```rust
use std::sync::Arc;
use synapse::mcp::HttpMcpClient;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> = Arc::new(DefaultHttpClient::new());
let mut client = HttpMcpClient::new("https://mcp.example.com", http);
client.initialize().await?;
let tools = client.list_tools().await?;
let result = client.call_tool("search", serde_json::json!({"q": "rust"})).await?;
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

Then pass to a `ChatModelAgent` or `GraphReActAgent`:

```rust
let agent = GraphReActAgent::builder()
    .name("assistant")
    .with_model(model)
    .with_tools(mcp_tools)
    .build()?;
```

## See Also

- [MCP Server how-to](../howto/mcp-server.md)
- [MCP HTTP test source](https://github.com/synapse-rs/synapse/blob/master/tests/mcp_http_test.rs)
