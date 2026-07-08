//! Hybrid Router/Handoff: combines LLM-driven routing with sub-agent handoff.
//!
//! The flow:
//! 1. LLM picks an initial sub-agent via tool call (router behavior)
//! 2. Sub-agent runs; if it returns `event.action.transfer_to`, the supervisor
//!    forces a call to the target without consulting the LLM (handoff behavior)
//! 3. Loop until no handoff and LLM returns a final text response
//!
//! Use cases:
//! - "I want to talk to billing" (LLM routes to billing)
//! - billing → realizes it's a refund question → transfers to refund
//!
//! This combines RouterSupervisor's LLM intelligence with HandoffSupervisor's
//! dynamic routing.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentOutput, AgentResult, ResumeInfo,
};
use crate::components::model::BaseChatModel;
use crate::schema::{Content, Message, ToolInfo};

const HYBRID_PROMPT_TEMPLATE: &str = include_str!("hybrid_system.md");

pub struct HybridSupervisor<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    agents: HashMap<String, Arc<dyn Agent>>,
    max_iterations: usize,
    max_handoffs: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HybridCheckpoint {
    messages: Vec<Message>,
    iteration: usize,
    handoff_count: usize,
    forced_target: Option<String>,
}

impl<M: BaseChatModel + 'static> HybridSupervisor<M> {
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> HybridSupervisorBuilder<M> {
        HybridSupervisorBuilder::new(name, model)
    }

    fn tool_infos(&self) -> Vec<ToolInfo> {
        self.agents
            .iter()
            .map(|(key, agent)| ToolInfo {
                name: format!("route_to_{}", key),
                description: agent.description(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task": {"type": "string", "description": "The task to delegate"}
                    },
                    "required": ["task"]
                }),
            })
            .collect()
    }

    fn system_prompt(&self) -> String {
        let agent_descs: Vec<String> = self
            .agents
            .iter()
            .map(|(k, a)| format!("- route_to_{}: {}", k, a.description()))
            .collect();
        HYBRID_PROMPT_TEMPLATE.replace("{agent_descriptions}", &agent_descs.join("\n"))
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for HybridSupervisor<M> {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let mut messages = vec![Message::system(self.system_prompt())];
        messages.extend(input.messages);
        let mut iteration = 0;
        let mut handoff_count = 0;
        let mut forced_target: Option<String> = None;

        loop {
            if iteration >= self.max_iterations {
                return Err(AgentError::MaxIterationsExceeded);
            }
            iteration += 1;

            let pending_calls: Vec<(String, String)> = if let Some(target) = forced_target.take() {
                vec![(format!("route_to_{}", target), format!("handoff_{}", target))]
            } else {
                let options = crate::components::model::GenerateOptions {
                    tools: Some(self.tool_infos()),
                    ..Default::default()
                };
                let mut response = self
                    .model
                    .generate(messages.clone(), options)
                    .await
                    .map_err(|e| AgentError::Agent(e.to_string()))?;

                let tool_calls_opt = response.tool_calls.take();
                match tool_calls_opt {
                    Some(tcs) if !tcs.is_empty() => {
                        messages.push(response);
                        tcs.into_iter()
                            .map(|tc| (tc.function.name, tc.id))
                            .collect()
                    }
                    _ => {
                        return Ok(AgentEvent {
                            agent_name: self.name.clone(),
                            output: AgentOutput {
                                message: Some(response),
                                action: None,
                            },
                            error: None,
                        });
                    }
                }
            };

            for (tool_name, tool_call_id) in pending_calls {
                let agent_key = tool_name
                    .strip_prefix("route_to_")
                    .unwrap_or(&tool_name)
                    .to_string();

                let agent = match self.agents.get(&agent_key) {
                    Some(a) => a.clone(),
                    None => {
                        messages.push(Message::tool(
                            format!("unknown agent: {}", agent_key),
                            tool_call_id,
                        ));
                        continue;
                    }
                };

                let sub_input = AgentInput {
                    messages: vec![Message::user(format!("[as {}] continue task", agent_key))],
                    enable_streaming: false,
                };
                let sub_event = agent.run(sub_input).await?;
                let sub_text = extract_text(&sub_event);
                messages.push(Message::tool(sub_text, tool_call_id));

                if let Some(target) = sub_event
                    .output
                    .action
                    .as_ref()
                    .and_then(|a| a.transfer_to.clone())
                {
                    if handoff_count >= self.max_handoffs {
                        return Err(AgentError::Agent(format!(
                            "max handoffs ({}) exceeded",
                            self.max_handoffs
                        )));
                    }
                    handoff_count += 1;
                    forced_target = Some(target);
                }
            }
        }
    }
    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        let state: HybridCheckpoint = serde_json::from_slice(&checkpoint)
            .map_err(|e| AgentError::Agent(format!("Invalid checkpoint: {e}")))?;
        let mut messages = state.messages;
        messages.push(Message::user(info.user_input));
        let mut iteration = state.iteration;
        let mut handoff_count = state.handoff_count;
        let mut forced_target = state.forced_target;

        loop {
            if iteration >= self.max_iterations {
                return Err(AgentError::MaxIterationsExceeded);
            }
            iteration += 1;

            let pending_calls: Vec<(String, String)> = if let Some(target) = forced_target.take() {
                vec![(format!("route_to_{}", target), format!("handoff_{}", target))]
            } else {
                let options = crate::components::model::GenerateOptions {
                    tools: Some(self.tool_infos()),
                    ..Default::default()
                };
                let mut response = self
                    .model
                    .generate(messages.clone(), options)
                    .await
                    .map_err(|e| AgentError::Agent(e.to_string()))?;

                let tool_calls_opt = response.tool_calls.take();
                match tool_calls_opt {
                    Some(tcs) if !tcs.is_empty() => {
                        messages.push(response);
                        tcs.into_iter()
                            .map(|tc| (tc.function.name, tc.id))
                            .collect()
                    }
                    _ => {
                        return Ok(AgentEvent {
                            agent_name: self.name.clone(),
                            output: AgentOutput {
                                message: Some(response),
                                action: None,
                            },
                            error: None,
                        });
                    }
                }
            };

            for (tool_name, tool_call_id) in pending_calls {
                let agent_key = tool_name
                    .strip_prefix("route_to_")
                    .unwrap_or(&tool_name)
                    .to_string();

                let agent = match self.agents.get(&agent_key) {
                    Some(a) => a.clone(),
                    None => {
                        messages.push(Message::tool(
                            format!("unknown agent: {}", agent_key),
                            tool_call_id,
                        ));
                        continue;
                    }
                };

                let sub_input = AgentInput {
                    messages: vec![Message::user(format!("[as {}] continue task", agent_key))],
                    enable_streaming: false,
                };
                let sub_event = agent.run(sub_input).await?;
                let sub_text = extract_text(&sub_event);
                messages.push(Message::tool(sub_text, tool_call_id));

                if let Some(target) = sub_event
                    .output
                    .action
                    .as_ref()
                    .and_then(|a| a.transfer_to.clone())
                {
                    if handoff_count >= self.max_handoffs {
                        return Err(AgentError::Agent(format!(
                            "max handoffs ({}) exceeded",
                            self.max_handoffs
                        )));
                    }
                    handoff_count += 1;
                    forced_target = Some(target);
                }
            }
        }
    }
}

fn extract_text(event: &AgentEvent) -> String {
    event
        .output
        .message
        .as_ref()
        .and_then(|m| match &m.content {
            Content::Text(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

pub struct HybridSupervisorBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    agents: HashMap<String, Arc<dyn Agent>>,
    max_iterations: usize,
    max_handoffs: usize,
}

impl<M: BaseChatModel + 'static> HybridSupervisorBuilder<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: "Hybrid Router/Handoff supervisor".to_string(),
            model,
            agents: HashMap::new(),
            max_iterations: 10,
            max_handoffs: 5,
        }
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    pub fn add_agent(mut self, key: impl Into<String>, agent: Arc<dyn Agent>) -> Self {
        self.agents.insert(key.into(), agent);
        self
    }
    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }
    pub fn max_handoffs(mut self, n: usize) -> Self {
        self.max_handoffs = n;
        self
    }
    pub fn build(self) -> HybridSupervisor<M> {
        HybridSupervisor {
            name: self.name,
            description: self.description,
            model: self.model,
            agents: self.agents,
            max_iterations: self.max_iterations,
            max_handoffs: self.max_handoffs,
        }
    }
}
