# Connect to MCP Server

Use external MCP tools (from any MCP-compatible server) in your
Synapse agents.

## Goal

Connect to an MCP server (Python, Node, or any other implementation),
list its tools, and use them in a Synapse agent.

## Stdio Transport (Local Subprocess)

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use synapse::mcp::StdioMcpClient;
use synapse::mcp::tool::McpToolAdapter;
use synapse::mcp::McpTool;
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Spawn MCP server subprocess
    let mut client = StdioMcpClient::spawn(
        "python3",
        &["my_mcp_server.py"],  // or "node", "server.js"
    )?;

    // 2. Initialize (MCP handshake)
    let init_result = client.initialize().await?;
    println!("Connected to: {}", init_result.server_info.name);

    // 3. List available tools
    let tools: Vec<McpTool> = client.list_tools().await?;
    println!("Available tools:");
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description);
    }

    // 4. Wrap tools as Synapse InvokableTool
    let client_arc = Arc::new(Mutex::new(client));
    let mcp_tools: Vec<Arc<McpToolAdapter>> = tools.into_iter()
        .map(|t| Arc::new(McpToolAdapter::new(t, client_arc.clone())))
        .collect();

    // 5. Build agent with MCP tools
    let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));
    let agent = GraphReActAgent::builder()
        .name("mcp-agent")
        .with_model(model)
        .with_tools(mcp_tools)
        .build()?;

    // 6. Use agent
    let event = agent.run(AgentInput {
        messages: vec![Message::user("Use the MCP tool to do X")],
        enable_streaming: false,
    }).await?;

    Ok(())
}
```

## HTTP Transport (Remote Server)

```rust
use std::sync::Arc;
use synapse::mcp::HttpMcpClient;
use synapse::transport::DefaultHttpClient;
use synapse::mcp::tool::McpToolAdapter;

let http: Arc<dyn synapse::transport::HttpClient> =
    Arc::new(DefaultHttpClient::new());
let mut client = HttpMcpClient::new("https://mcp.example.com", http);
client.initialize().await?;
let tools = client.list_tools().await?;

// Wrap and use as above
```

## Writing a Mock MCP Server (for Testing)

A minimal Python MCP server:

```python
import sys
import json

def send(obj):
    body = json.dumps(obj)
    sys.stdout.write(f"Content-Length: {len(body)}\r\n\r\n{body}")
    sys.stdout.flush()

def read_message():
    headers = {}
    while True:
        line = sys.stdin.readline().strip()
        if not line:
            break
        k, v = line.split(":", 1)
        headers[k.strip()] = v.strip()
    body = sys.stdin.read(int(headers["Content-Length"]))
    return json.loads(body)

# Handle initialize
init_request = read_message()
send({
    "jsonrpc": "2.0",
    "id": init_request["id"],
    "result": {
        "protocolVersion": "2024-11-05",
        "serverInfo": {"name": "mock", "version": "1.0"},
        "capabilities": {}
    }
})

# Handle tools/list
list_request = read_message()
send({
    "jsonrpc": "2.0",
    "id": list_request["id"],
    "result": {
        "tools": [
            {"name": "echo", "description": "Echoes input", "inputSchema": {"type": "object"}}
        ]
    }
})

# Handle tools/call
call_request = read_message()
tool_name = call_request["params"]["name"]
text = call_request["params"]["arguments"].get("text", "")
send({
    "jsonrpc": "2.0",
    "id": call_request["id"],
    "result": {
        "content": [{"type": "text", "text": f"Echo: {text}"}],
        "isError": False
    }
})
```

## See Also

- [MCP concepts](../concepts/mcp.md)
- [A2A how-to](./a2a-cross-process.md) — For agent-to-agent communication
