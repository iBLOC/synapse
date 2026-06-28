use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use synapse::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo,
};
use synapse::adk::multiagent::AgentTool;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError};
use synapse::schema::{Content, Message};

struct MockAgent {
    name: String,
    description: String,
    response: String,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

impl MockAgent {
    fn new(name: &str, description: &str, response: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            description: description.to_string(),
            response: response.to_string(),
            received_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl Agent for MockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        let task = input
            .messages
            .last()
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.received_tasks.lock().unwrap().push(task);
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant(self.response.clone())),
                action: None,
            },
            error: None,
        })
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_agent_tool_info_has_route_to_prefix() {
    let agent = MockAgent::new("researcher", "Researches topics", "answer");
    let tool = AgentTool::new(agent.clone());

    let info = tool.info().await.unwrap();
    assert_eq!(info.name, "route_to_researcher");
    assert_eq!(info.description, "Researches topics");
}

#[tokio::test]
async fn test_agent_tool_info_has_task_parameter() {
    let agent = MockAgent::new("coder", "Writes code", "code");
    let tool = AgentTool::new(agent);

    let info = tool.info().await.unwrap();
    let params = info.parameters.as_object().expect("params should be object");
    assert_eq!(params.get("type").and_then(|v| v.as_str()), Some("object"));
    let properties = params
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("properties");
    assert!(properties.contains_key("task"));
    let required = params
        .get("required")
        .and_then(|v| v.as_array())
        .expect("required");
    assert!(required.iter().any(|v| v.as_str() == Some("task")));
}
