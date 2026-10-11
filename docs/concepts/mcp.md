---
title: MCP (Model Context Protocol)
---

# MCP (Model Context Protocol) · 模型上下文协议

MCP is a standard protocol for connecting LLMs to tools and data sources.
Synapse implements both MCP client and server.
*MCP 是连接 LLM 与工具、数据源的标准协议。Synapse 同时实现了 MCP client 与 server。*

## What is MCP? · 什么是 MCP？

MCP defines a JSON-RPC based protocol where:
*MCP 定义了一套基于 JSON-RPC 的协议：*

- **Server** exposes tools, resources, prompts
  *Server 端暴露 tools、resources、prompts*
- **Client** invokes them, gets results
  *Client 端调用它们并获取结果*

## Transports · 传输方式

| Transport / 传输 | Use Case / 适用场景 | Class / 实现类 |
|-----------|----------|-------|
| **stdio** | Local subprocess (Python, Node, etc.) / 本地子进程（Python、Node 等） | `StdioMcpClient` |
| **HTTP** | Remote MCP server / 远程 MCP server | `HttpMcpClient` |

Both implement the same `initialize / list_tools / call_tool` API.
*两者实现同一套 `initialize / list_tools / call_tool` API。*

## MCP Client · MCP 客户端

### StdioMcpClient · Stdio 客户端

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn("python3", &["my_mcp_server.py"])?;
let init = client.initialize().await?;
let tools = client.list_tools().await?;
let result = client.call_tool("echo", serde_json::json!({"text": "hi"})).await?;
```

### HttpMcpClient · HTTP 客户端

```rust
use std::sync::Arc;
use synapse::mcp::HttpMcpClient;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> = Arc::new(DefaultHttpClient::new());
let mut client = HttpMcpClient::new("https://mcp.example.com", http);
client.initialize().await?;
let tools = client.list_tools().await?;
```

## MCP Server · MCP 服务端

Synapse provides a trait-based MCP server abstraction:
*Synapse 提供基于 trait 的 MCP server 抽象：*

### McpServer Trait · MCP 服务器 trait

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

### SimpleMcpServer · 简易 MCP 服务器

Build an MCP server with handler functions:
*用 handler 函数构建 MCP 服务器：*

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

## Using MCP Tools in Synapse Agents · 在 Synapse Agent 中使用 MCP 工具

Wrap MCP tools as Synapse `InvokableTool` with `McpToolAdapter`:
*用 `McpToolAdapter` 把 MCP 工具包装成 Synapse 的 `InvokableTool`：*

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

## See Also · 相关阅读

- [MCP Server how-to](/howto/mcp-server)
