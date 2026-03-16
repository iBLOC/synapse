use crate::adk::agent::{Agent, AgentInput, AgentEvent, AgentError};

pub struct Runner {
    agent: Box<dyn Agent>,
}

impl Runner {
    pub fn new(agent: Box<dyn Agent>) -> Self {
        Self { agent }
    }
    
    pub async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        self.agent.run(input).await
    }
}
