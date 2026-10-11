---
title: Tool
---

# 工具

工具扩展 LLM agent 的能力。LLM 可以在响应中发出 `tool_call` 来决定"调用"某个工具；agent runtime 随后执行该工具，并把结果回传给 LLM。

## Trait 层级

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

## 工具信息 Schema

工具的 `info()` 返回 `ToolInfo`，描述其 name、description 以及参数的 JSON Schema：

```rust
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}
```

## 内置工具来源

| 来源 | 说明 |
|--------|-------------|
| Custom `InvokableTool` | 用户自定义工具（Calculator、Search 等） |
| `McpToolAdapter` | 包装 MCP 工具（stdio 或 HTTP transport） |
| `AgentTool` | 把任意 `Agent` 包装成工具（用于 MultiAgent） |

## 工具中间件

`ChatModelAgent` 通过 `ToolMiddleware` 支持 tool 执行前/后的 hook：

```rust
pub trait ToolMiddleware: Send + Sync {
    fn before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError>;
    fn after_tool(&self, name: &str, result: &str) -> Result<(), ToolError>;
}

let agent = ChatModelAgent::builder()
    .with_tool_middleware(Arc::new(LoggingToolMiddleware::new()))
    .build();
```

## 示例：HTTP 工具

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
                    "properties": {"url": {"type": "string"}},
                    "required": ["url"]
                }),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for HttpGetTool {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        // Parse args, call client.execute(), return body
        Ok("response".to_string())
    }
}
```

`McpToolAdapter`、`AgentTool` 等在内部都使用这一模式。
