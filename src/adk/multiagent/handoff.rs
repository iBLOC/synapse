//! Conversational handoff: sub-agents can hand off a conversation to one
//! another by returning an `AgentEvent` with `output.action.transfer_to`
//! set to the target agent's name.
//!
//! `HandoffSupervisor` is a non-LLM coordinator that runs an entry agent
//! and routes to subsequent agents based on transfer signals. Useful for
//! multi-role conversations (customer service, multi-expert systems).

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo,
};

pub struct HandoffSupervisor {
    name: String,
    description: String,
    agents: HashMap<String, Arc<dyn Agent>>,
    entry_point: String,
    max_handoffs: usize,
}

impl HandoffSupervisor {
    pub fn builder(name: impl Into<String>) -> HandoffSupervisorBuilder {
        HandoffSupervisorBuilder::new(name)
    }
}

#[async_trait]
impl Agent for HandoffSupervisor {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        let mut current = self.entry_point.clone();
        let mut messages = input.messages;
        let mut handoff_count: usize = 0;

        loop {
            let agent = self.agents.get(&current).ok_or_else(|| {
                AgentError::Agent(format!("handoff target '{}' not found", current))
            })?;

            let event = agent
                .run(AgentInput {
                    messages: messages.clone(),
                    enable_streaming: false,
                })
                .await?;

            let transfer_target = event
                .output
                .action
                .as_ref()
                .and_then(|a| a.transfer_to.clone());

            match transfer_target {
                Some(target) => {
                    handoff_count += 1;
                    if handoff_count > self.max_handoffs {
                        return Err(AgentError::Agent(format!(
                            "max handoffs ({}) exceeded",
                            self.max_handoffs
                        )));
                    }
                    if let Some(msg) = &event.output.message {
                        messages.push(msg.clone());
                    }
                    current = target;
                }
                None => return Ok(event),
            }
        }
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct HandoffSupervisorBuilder {
    name: String,
    description: String,
    agents: HashMap<String, Arc<dyn Agent>>,
    entry_point: Option<String>,
    max_handoffs: usize,
}

impl HandoffSupervisorBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: "Handoff supervisor".to_string(),
            agents: HashMap::new(),
            entry_point: None,
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

    pub fn entry_point(mut self, name: impl Into<String>) -> Self {
        self.entry_point = Some(name.into());
        self
    }

    pub fn max_handoffs(mut self, n: usize) -> Self {
        self.max_handoffs = n;
        self
    }

    pub fn build(self) -> HandoffSupervisor {
        let entry = self
            .entry_point
            .or_else(|| self.agents.keys().next().cloned())
            .expect("HandoffSupervisor requires at least one agent");
        HandoffSupervisor {
            name: self.name,
            description: self.description,
            agents: self.agents,
            entry_point: entry,
            max_handoffs: self.max_handoffs,
        }
    }
}
