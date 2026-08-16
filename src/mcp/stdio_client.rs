use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};

use super::types::{
    JsonRpcRequest, JsonRpcResponse, McpCallToolParams, McpCallToolResult, McpClientCapabilities,
    McpClientInfo, McpInitializeParams, McpInitializeResult, McpListToolsResult, McpTool,
};
use super::{McpError, McpResult};

pub struct StdioMcpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: Mutex<i64>,
}

impl StdioMcpClient {
    pub fn from_child(mut child: Child) -> McpResult<Self> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Io("no stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Io("no stdout".to_string()))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: Mutex::new(1),
        })
    }

    pub fn spawn(command: &str, args: &[&str]) -> McpResult<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = cmd
            .spawn()
            .map_err(|e| McpError::Io(format!("spawn failed: {}", e)))?;
        Self::from_child(child)
    }

    pub fn initialize(&mut self) -> McpResult<McpInitializeResult> {
        let params = McpInitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpClientCapabilities::default(),
            client_info: McpClientInfo {
                name: "synapse".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        let result = self.request("initialize", Some(serde_json::to_value(&params).unwrap()))?;
        let init: McpInitializeResult = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("initialize: {}", e)))?;
        self.notify("notifications/initialized", Some(json!({})))?;
        Ok(init)
    }

    pub fn list_tools(&mut self) -> McpResult<Vec<McpTool>> {
        let result = self.request("tools/list", Some(json!({})))?;
        let parsed: McpListToolsResult = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("list_tools: {}", e)))?;
        Ok(parsed.tools)
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value) -> McpResult<McpCallToolResult> {
        let params = McpCallToolParams {
            name: name.to_string(),
            arguments: Some(arguments),
        };
        let result = self.request("tools/call", Some(serde_json::to_value(&params).unwrap()))?;
        serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(format!("call_tool: {}", e)))
    }

    fn next_id(&self) -> i64 {
        let mut id = self.next_id.lock().unwrap();
        let cur = *id;
        *id += 1;
        cur
    }

    fn request(&mut self, method: &str, params: Option<Value>) -> McpResult<Value> {
        let id = self.next_id();
        let req = JsonRpcRequest {
            jsonrpc: super::types::JSONRPC_VERSION.to_string(),
            id: Value::Number(id.into()),
            method: method.to_string(),
            params,
        };
        self.write_message(&req)?;
        self.read_response()
    }

    fn notify(&mut self, method: &str, params: Option<Value>) -> McpResult<()> {
        let req = JsonRpcRequest {
            jsonrpc: super::types::JSONRPC_VERSION.to_string(),
            id: Value::Null,
            method: method.to_string(),
            params,
        };
        self.write_message(&req)?;
        Ok(())
    }

    fn write_message(&mut self, req: &JsonRpcRequest) -> McpResult<()> {
        let body = serde_json::to_string(req)
            .map_err(|e| McpError::InvalidResponse(format!("serialize: {}", e)))?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin
            .write_all(header.as_bytes())
            .map_err(|e| McpError::Io(format!("write header: {}", e)))?;
        self.stdin
            .write_all(body.as_bytes())
            .map_err(|e| McpError::Io(format!("write body: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| McpError::Io(format!("flush: {}", e)))?;
        Ok(())
    }

    fn read_response(&mut self) -> McpResult<Value> {
        let headers = self.read_headers()?;
        let mut body = vec![0u8; headers.get("Content-Length").copied().unwrap_or(0)];
        if !body.is_empty() {
            use std::io::Read;
            self.stdout
                .read_exact(&mut body)
                .map_err(|e| McpError::Io(format!("read body: {}", e)))?;
        }
        let resp: JsonRpcResponse = serde_json::from_slice(&body)
            .map_err(|e| McpError::InvalidResponse(format!("parse: {}", e)))?;
        if let Some(err) = resp.error {
            return Err(McpError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        resp.result.ok_or(McpError::ConnectionClosed)
    }

    fn read_headers(&mut self) -> McpResult<HashMap<String, usize>> {
        let mut headers: HashMap<String, usize> = HashMap::new();
        loop {
            let mut line = String::new();
            let n = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| McpError::Io(format!("read_line: {}", e)))?;
            if n == 0 {
                return Err(McpError::ConnectionClosed);
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if let Ok(n) = v.trim().parse::<usize>() {
                    headers.insert(k.trim().to_string(), n);
                }
            }
        }
        Ok(headers)
    }

    pub fn shutdown(&mut self) -> McpResult<()> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        Ok(())
    }
}

impl Drop for StdioMcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
