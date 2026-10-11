---
title: MCP (Model Context Protocol)
---

# 模型上下文协议

MCP 是连接 LLM 与工具、数据源的标准协议。Synapse 同时实现了 MCP client 与 server。

## 什么是 MCP？

MCP 定义了一套基于 JSON-RPC 的协议：

  - Server 端暴露 tools、resources、prompts
  - Client 端调用它们并获取结果

## 传输方式

| 传输 | 适用场景 | 实现类 |
|-----------|----------|-------|
| **stdio** | 本地子进程（Python、Node 等） | `StdioMcpClient` |
| **HTTP** | 远程 MCP server | `HttpMcpClient` |

两者实现同一套 `initialize / list_tools / call_tool` API。

## MCP 客户端

### Stdio 客户端

```rust
use synapse::mcp::StdioMcpClient;

let mut client = StdioMcpClient::spawn("python3", &["my_mcp_server.py"])?;
let init = client.initialize().await?;
let tools = client.list_tools().await?;
let result = client.call_tool("echo", serde_json::json!({"text": "hi"})).await?;
```

### HTTP 客户端

```rust
use std::sync::Arc;
use synapse::mcp::HttpMcpClient;
use synapse::transport::DefaultHttpClient;

let http: Arc<dyn synapse::transport::HttpClient> = Arc::new(DefaultHttpClient::new());
let mut client = HttpMcpClient::new("https://mcp.example.com", http);
client.initialize().await?;
let tools = client.list_tools().await?;
```

## MCP 服务端

Synapse 提供基于 trait 的 MCP server 抽象：

### MCP 服务器 trait

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

### 简易 MCP 服务器

用 handler 函数构建 MCP 服务器：

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

## 在 Synapse Agent 中使用 MCP 工具

用 `McpToolAdapter` 把 MCP 工具包装成 Synapse 的 `InvokableTool`：

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

## 相关阅读
