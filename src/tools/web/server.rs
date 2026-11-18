//! 把这一层**出**成 MCP server —— 端脑注册它就能用。
//!
//! 端脑那侧是**数据面**注册（`McpServerRecord.transport` = stdio／http）
//! ⇒ ⇒ 这一层只要能给出一个能跑的 MCP server，端脑就接得上，
//! 而**模型看到的是我们自己的三个名字**（`web_*`），不是引擎那 38 个。

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::mcp::types::{McpCallToolResult, McpContent, McpTextContent};
use crate::mcp::server::{SimpleMcpServer, ToolHandler};
use crate::mcp::McpResult;
use crate::tools::Tool;

use super::{WebActTool, WebFetchTool, WebReadTool, WebSession};

/// 造一个「shadow-web」MCP server（三个工具）。
pub fn mcp_server(session: Arc<WebSession>) -> SimpleMcpServer {
    // ⚠️ 名字／描述／schema **先取成 owned** —— 否则 borrow 之后没法把工具移进去。
    let fetch = WebFetchTool::new(session.clone());
    let (fetch_name, fetch_desc, fetch_schema) =
        (fetch.name().to_string(), fetch.description().to_string(), fetch.input_schema());
    let read = WebReadTool::new(session.clone());
    let (read_name, read_desc, read_schema) =
        (read.name().to_string(), read.description().to_string(), read.input_schema());
    let act = WebActTool::new(session);
    let (act_name, act_desc, act_schema) =
        (act.name().to_string(), act.description().to_string(), act.input_schema());
    SimpleMcpServer::new("shadow-web", env!("CARGO_PKG_VERSION"))
        .with_tool(
            fetch_name,
            fetch_desc,
            Some(fetch_schema),
            AnyTool::Fetch(fetch),
        )
        .with_tool(read_name, read_desc, Some(read_schema), AnyTool::Read(read))
        .with_tool(act_name, act_desc, Some(act_schema), AnyTool::Act(act))
}

/// 三个工具的共同外壳（把 `Tool` 的结果转成 MCP 的结果）。
enum AnyTool {
    Fetch(WebFetchTool),
    Read(WebReadTool),
    Act(WebActTool),
}

#[async_trait]
impl ToolHandler for AnyTool {
    async fn execute(&self, arguments: Value) -> McpResult<McpCallToolResult> {
        let outcome = match self {
            AnyTool::Fetch(t) => t.execute(arguments).await,
            AnyTool::Read(t) => t.execute(arguments).await,
            AnyTool::Act(t) => t.execute(arguments).await,
        };
        // ⚠️ MCP 的约定：工具**执行失败**用 `isError: true` 的结果回报，
        // 不是 JSON-RPC 错误 ⇒ ⇒ 模型能读到失败原因，而不是「调用炸了」。
        let (text, is_error) = match outcome {
            Ok(out) => (out.content, out.is_error),
            Err(err) => (err.to_string(), true),
        };
        Ok(McpCallToolResult {
            content: vec![McpContent::Text(McpTextContent { text })],
            is_error,
        })
    }
}
