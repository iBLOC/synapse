//! Host/Orchestrator pattern: decomposes a task into sub-tasks and dispatches
//! them to specialized sub-agents in parallel via LLM tool calling.
//!
//! Implementation: `OrchestratorAgent` wraps `RouterSupervisor` and uses an
//! orchestrator-specific system prompt to guide the LLM toward parallel
//! dispatch. No new run loop is introduced.

use std::sync::Arc;

use async_trait::async_trait;

use crate::adk::agent::{
    Agent, AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo,
};
use crate::adk::multiagent::agent_tool::AgentTool;
use crate::adk::multiagent::router::RouterSupervisor;
use crate::components::model::BaseChatModel;

const ORCHESTRATOR_PROMPT_TEMPLATE: &str = include_str!("orchestrator_system.md");

pub struct OrchestratorAgent<M: BaseChatModel + 'static> {
    inner: RouterSupervisor<M>,
    name: String,
}

impl<M: BaseChatModel + 'static> OrchestratorAgent<M> {
    pub fn builder(name: impl Into<String>, model: Arc<M>) -> OrchestratorAgentBuilder<M> {
        OrchestratorAgentBuilder::new(name, model)
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for OrchestratorAgent<M> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.inner.description()
    }

    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
        self.inner.run(input).await
    }

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        self.inner.resume(checkpoint, info).await
    }
}

pub struct OrchestratorAgentBuilder<M: BaseChatModel + 'static> {
    name: String,
    description: String,
    model: Arc<M>,
    agents: Vec<Arc<dyn Agent>>,
    max_iterations: usize,
}

impl<M: BaseChatModel + 'static> OrchestratorAgentBuilder<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: "Orchestrator agent".to_string(),
            model,
            agents: Vec::new(),
            max_iterations: 12,
        }
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn add_agent(mut self, agent: Arc<dyn Agent>) -> Self {
        self.agents.push(agent);
        self
    }

    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    pub fn build(self) -> OrchestratorAgent<M> {
        let agent_descriptions: Vec<String> = self
            .agents
            .iter()
            .map(|a| {
                format!(
                    "- route_to_{}: {}",
                    a.name(),
                    a.description()
                )
            })
            .collect();
        let prompt = ORCHESTRATOR_PROMPT_TEMPLATE
            .replace("{agent_descriptions}", &agent_descriptions.join("\n"));

        let mut router_builder = RouterSupervisor::builder(self.name.clone(), self.model)
            .description(self.description)
            .max_iterations(self.max_iterations)
            .system_prompt(prompt);

        for agent in self.agents {
            router_builder = router_builder.add_agent(agent);
        }
        let router = router_builder.build();

        OrchestratorAgent {
            inner: router,
            name: self.name,
        }
    }
}
