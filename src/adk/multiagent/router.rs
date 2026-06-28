// Placeholder — full implementation in Chunk 2.
// Provides stub types so the multiagent module compiles before Chunk 2 lands.

use std::sync::Arc;

use crate::adk::agent::{AgentError, AgentEvent, AgentInput, AgentResult, ResumeInfo};
use crate::components::model::BaseChatModel;

pub struct RouterSupervisor<M: BaseChatModel + 'static> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M: BaseChatModel + 'static> RouterSupervisor<M> {
    pub fn builder(_name: impl Into<String>, _model: Arc<M>) -> RouterSupervisorBuilder<M> {
        RouterSupervisorBuilder::new()
    }
}

pub struct RouterSupervisorBuilder<M: BaseChatModel + 'static> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M: BaseChatModel + 'static> RouterSupervisorBuilder<M> {
    pub fn new() -> Self {
        Self { _phantom: std::marker::PhantomData }
    }
    pub fn description(self, _d: impl Into<String>) -> Self { self }
    pub fn add_agent(self, _agent: Arc<dyn crate::adk::agent::Agent>) -> Self { self }
    pub fn max_iterations(self, _n: usize) -> Self { self }
    pub fn system_prompt(self, _p: impl Into<String>) -> Self { self }
    pub fn build(self) -> RouterSupervisor<M> {
        RouterSupervisor { _phantom: std::marker::PhantomData }
    }
}

#[async_trait::async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> crate::adk::agent::Agent for RouterSupervisor<M> {
    fn name(&self) -> String { "stub".into() }
    fn description(&self) -> String { "stub".into() }
    async fn run(&self, _input: AgentInput) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("not yet implemented".into()))
    }
    async fn resume(&self, _checkpoint: Vec<u8>, _info: ResumeInfo) -> AgentResult<AgentEvent> {
        Err(AgentError::Agent("not yet implemented".into()))
    }
}
