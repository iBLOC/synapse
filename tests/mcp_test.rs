use std::io::Write;
use std::process::{Command, Stdio};
use synapse::mcp::{McpContent, StdioMcpClient};

fn start_mock_server() -> std::process::Child {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    Command::new(&python)
        .arg("-c")
        .arg(MOCK_SERVER_SCRIPT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn mock MCP server")
}

const MOCK_SERVER_SCRIPT: &str = r#"
import json, sys

def read_message():
    headers = {}
    while True:
        line = sys.stdin.readline()
        if not line:
            return None
        line = line.rstrip('\r\n')
        if line == '':
            break
        if ':' in line:
            k, v = line.split(':', 1)
            headers[k.strip()] = v.strip()
    length = int(headers.get('Content-Length', '0'))
    if length == 0:
        return None
    body = sys.stdin.read(length)
    return json.loads(body)

def send_message(msg):
    body = json.dumps(msg)
    sys.stdout.write(f'Content-Length: {len(body)}\r\n\r\n')
    sys.stdout.write(body)
    sys.stdout.flush()

tools = [
    {"name": "echo", "description": "echo back input",
     "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}},
    {"name": "add", "description": "add two numbers",
     "inputSchema": {"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}, "required": ["a", "b"]}},
]

while True:
    req = read_message()
    if req is None:
        break
    method = req.get('method')
    rid = req.get('id')
    if method == 'initialize':
        send_message({"jsonrpc": "2.0", "id": rid, "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "mock", "version": "0.1.0"}
        }})
    elif method == 'notifications/initialized':
        pass
    elif method == 'tools/list':
        send_message({"jsonrpc": "2.0", "id": rid, "result": {"tools": tools}})
    elif method == 'tools/call':
        params = req.get('params', {})
        name = params.get('name')
        args = params.get('arguments', {})
        if name == 'echo':
            text = args.get('text', '')
            send_message({"jsonrpc": "2.0", "id": rid, "result": {
                "content": [{"type": "text", "text": text}],
                "isError": False
            }})
        elif name == 'add':
            a = args.get('a', 0)
            b = args.get('b', 0)
            send_message({"jsonrpc": "2.0", "id": rid, "result": {
                "content": [{"type": "text", "text": str(a + b)}],
                "isError": False
            }})
        else:
            send_message({"jsonrpc": "2.0", "id": rid, "error": {"code": -32601, "message": f"unknown tool: {name}"}})
    else:
        send_message({"jsonrpc": "2.0", "id": rid, "error": {"code": -32601, "message": "method not found"}})
"#;

#[test]
fn test_mcp_initialize_and_list_tools() {
    let child = start_mock_server();
    let mut client = StdioMcpClient::from_child(child).expect("from_child");
    let init = client.initialize().expect("initialize");
    assert_eq!(init.server_info.name, "mock");

    let tools = client.list_tools().expect("list_tools");
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().any(|t| t.name == "echo"));
    assert!(tools.iter().any(|t| t.name == "add"));
}

#[test]
fn test_mcp_call_tool_echo() {
    let child = start_mock_server();
    let mut client = StdioMcpClient::from_child(child).expect("from_child");
    client.initialize().expect("initialize");

    let args = serde_json::json!({"text": "hello world"});
    let result = client.call_tool("echo", args).expect("call_tool");
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        McpContent::Text(t) => assert_eq!(t.text, "hello world"),
    }
}

#[test]
fn test_mcp_call_tool_add() {
    let child = start_mock_server();
    let mut client = StdioMcpClient::from_child(child).expect("from_child");
    client.initialize().expect("initialize");

    let args = serde_json::json!({"a": 3, "b": 4});
    let result = client.call_tool("add", args).expect("call_tool");
    match &result.content[0] {
        McpContent::Text(t) => assert_eq!(t.text, "7"),
    }
}
