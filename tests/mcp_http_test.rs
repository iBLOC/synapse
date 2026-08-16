use std::sync::Arc;

use synapse::mcp::HttpMcpClient;
use synapse::transport::{DefaultHttpClient, HttpClient};

#[tokio::test]
async fn test_http_mcp_initialize() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/mcp")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {"name": "test-server", "version": "1.0"},
                    "capabilities": {}
                }
            }"#,
        )
        .create_async()
        .await;

    let http: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let mut client = HttpMcpClient::new(server.url(), http);
    let result = client.initialize().await.expect("init failed");
    assert_eq!(result.server_info.name, "test-server");
    assert_eq!(result.server_info.version, "1.0");
    assert_eq!(result.protocol_version, "2024-11-05");
}

#[tokio::test]
async fn test_http_mcp_list_tools() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/mcp")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "tools": [
                        {"name": "echo", "description": "Echoes input", "inputSchema": {"type": "object"}},
                        {"name": "add", "description": "Adds numbers", "inputSchema": {"type": "object"}}
                    ]
                }
            }"#,
        )
        .create_async()
        .await;

    let http: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let client = HttpMcpClient::new(server.url(), http);
    let tools = client.list_tools().await.expect("list failed");
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "echo");
    assert_eq!(tools[1].name, "add");
}

#[tokio::test]
async fn test_http_mcp_call_tool() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/mcp")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "content": [{"type": "text", "text": "42"}],
                    "isError": false
                }
            }"#,
        )
        .create_async()
        .await;

    let http: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let client = HttpMcpClient::new(server.url(), http);
    let result = client
        .call_tool("add", serde_json::json!({"a": 1, "b": 2}))
        .await
        .expect("call failed");
    assert!(!result.is_error);
    let text = match &result.content[0] {
        synapse::mcp::McpContent::Text(t) => &t.text,
    };
    assert_eq!(text, "42");
}

#[tokio::test]
async fn test_http_mcp_rpc_error() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/mcp")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "jsonrpc": "2.0",
                "id": 1,
                "error": {"code": -32601, "message": "method not found"}
            }"#,
        )
        .create_async()
        .await;

    let http: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let client = HttpMcpClient::new(server.url(), http);
    let result = client.list_tools().await;
    assert!(matches!(result, Err(synapse::mcp::McpError::Rpc { code: -32601, .. })));
}

#[tokio::test]
async fn test_http_mcp_http_error() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("POST", "/mcp")
        .with_status(500)
        .with_body("internal error")
        .create_async()
        .await;

    let http: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let client = HttpMcpClient::new(server.url(), http);
    let result = client.list_tools().await;
    assert!(matches!(result, Err(synapse::mcp::McpError::Io(_))));
}
