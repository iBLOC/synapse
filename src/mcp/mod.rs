pub mod http_client;
pub mod server;
pub mod stdio_client;
pub mod tool;
pub mod types;

pub use http_client::HttpMcpClient;
pub use server::{McpServer, SimpleMcpServer, ToolHandler};
pub use stdio_client::StdioMcpClient;
pub use tool::McpToolAdapter;
pub use types::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpCallToolParams, McpCallToolResult,
    McpClientCapabilities, McpClientInfo, McpContent, McpInitializeParams, McpInitializeResult,
    McpListToolsResult, McpServerInfo, McpTextContent, McpTool,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("JSON-RPC error: code={code}, message={message}")]
    Rpc { code: i32, message: String },
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("connection closed")]
    ConnectionClosed,
}

pub type McpResult<T> = Result<T, McpError>;
