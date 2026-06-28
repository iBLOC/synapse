//! Router/Supervisor pattern: an LLM-powered host that delegates to sub-agents
//! via tool calling.
//!
//! See `docs/superpowers/specs/2026-06-28-multiagent-router-design.md` for the
//! full design rationale.

use std::sync::Arc;

use async_trait::async_trait;

use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo};
use crate::adk::multiagent::agent_tool::AgentTool;
use crate::adk::react::ChatModelAgent;
use crate::components::model::BaseChatModel;

const DEFAULT_SYSTEM_PROMPT_TEMPLATE: &str = include_str!("supervisor_system.md");

pub struct RouterSupervisor<M: BaseChatModel + 'static> {
    inner: ChatModelAgent<M, AgentTool>,
    name: String,
    description: String,
}

impl<M: BaseChatModel + 'static> RouterSupervisor<M> {
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> RouterSupervisorBuilder<M> {
        RouterSupervisorBuilder::new(name, model)
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for RouterSupervisor<M> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        self.inner.run(input).await
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct RouterSupervisorBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<AgentTool>>,
    max_iterations: usize,
    system_prompt: Option<String>,
}

impl<M: BaseChatModel + 'static> RouterSupervisorBuilder<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: "Router/Supervisor agent".to_string(),
            model,
            tools: Vec::new(),
            max_iterations: 8,
            system_prompt: None,
        }
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn add_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.tools.push(Arc::new(AgentTool::new(agent)));
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn system_prompt(mut self, p: impl Into<String>) -> Self {
        self.system_prompt = Some(p.into());
        self
    }

    pub fn build(self) -> RouterSupervisor<M> {
        let agent_descriptions: Vec<String> = self
            .tools
            .iter()
            .map(|t| format!("- route_to_{}: {}", t.agent_name(), "<see tool description>"))
            .collect();

        let prompt = match self.system_prompt {
            Some(p) => p,
            None => DEFAULT_SYSTEM_PROMPT_TEMPLATE.replace(
                "{agent_descriptions}",
                &agent_descriptions.join("\n"),
            ),
        };

        let inner = ChatModelAgent::new(
            self.name.clone(),
            self.description.clone(),
            self.model,
        )
        .with_tools(self.tools)
        .max_iterations(self.max_iterations)
        .with_system_prompt(prompt);

        RouterSupervisor {
            inner,
            name: self.name,
            description: self.description,
        }
    }
}
