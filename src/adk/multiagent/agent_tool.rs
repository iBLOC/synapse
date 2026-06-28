//! Bridge between Agent trait and InvokableTool.
//!
//! Wraps any `Arc<dyn Agent>` as a `Tool` so a supervisor LLM can route to
//! sub-agents via tool calls (more reliable than free-form text routing).

use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;

use crate::adk::agent::{Agent, AgentInput};
use crate::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use crate::schema::{Content, Message, ToolInfo};

pub struct AgentTool {
    agent: Arc<dyn Agent>,
    cached_info: ToolInfo,
}

impl AgentTool {
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        let cached_info = ToolInfo {
            name: format!("route_to_{}", agent.name()),
            description: agent.description(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The task to delegate to this agent"
                    }
                },
                "required": ["task"]
            }),
        };
        Self { agent, cached_info }
    }

    pub fn agent_name(&self) -> String {
        self.agent.name()
    }
}

impl BaseTool for AgentTool {
    fn info(&self) -> impl Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move { Ok(self.cached_info.clone()) }
    }
}

#[async_trait]
impl InvokableTool for AgentTool {
    async fn invoke(&self, arguments: String) -> ToolResult_ {
        let args: serde_json::Value = serde_json::from_str(&arguments)
            .map_err(|e| ToolError::InvalidArguments(format!("parse arguments: {}", e)))?;
        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("missing 'task' field".into()))?;

        let input = AgentInput {
            messages: vec![Message::user(task.to_string())],
            enable_streaming: false,
        };

        let event = match self.agent.run(input).await {
            Ok(e) => e,
            Err(e) => {
                return Ok(serde_json::json!({
                    "agent": self.agent.name(),
                    "success": false,
                    "error": e.to_string(),
                })
                .to_string());
            }
        };

        let text = event
            .output
            .message
            .as_ref()
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "agent": self.agent.name(),
            "success": event.error.is_none(),
            "output": text,
        })
        .to_string())
    }
}
