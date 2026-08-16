use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput};
use crate::components::model::BaseChatModel;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize)]
enum DeepPhase {
    Decompose,
    Execute {
        index: usize,
        subtasks: Vec<String>,
        results: Vec<String>,
        messages: Vec<crate::schema::Message>,
    },
    Synthesize { results: String, task: String },
}

pub struct DeepAgent<M: BaseChatModel> {
    name: String,
    description: String,
    model: Arc<M>,
    sub_agents: Vec<Box<dyn Agent>>,
}

impl<M: BaseChatModel + 'static> DeepAgent<M> {
    pub fn new(name: impl Into<String>, description: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            model,
            sub_agents: Vec::new(),
        }
    }

    pub fn with_sub_agents(mut self, agents: Vec<Box<dyn Agent>>) -> Self {
        self.sub_agents = agents;
        self
    }

    async fn decompose_task(&self, task: &str) -> Result<Vec<String>, AgentError> {
        let prompt = format!(
            "Decompose the following task into subtasks:\n\nTask: {}\n\nProvide a list of subtasks, one per line.",
            task
        );

        let message = crate::schema::Message::user(prompt);
        let options = crate::components::model::GenerateOptions::default();

        let response = self
            .model
            .generate(vec![message], options)
            .await
            .map_err(|e| AgentError::Agent(e.to_string()))?;

        let content = match response.content {
            crate::schema::Content::Text(s) => s,
            _ => return Err(AgentError::Agent("Invalid response".to_string())),
        };

        let subtasks: Vec<String> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        Ok(subtasks)
    }

    async fn execute_subtasks(
        &self,
        subtasks: Vec<String>,
        messages: &mut Vec<crate::schema::Message>,
    ) -> Result<String, AgentError> {
        let mut results = Vec::new();

        for (i, subtask) in subtasks.iter().enumerate() {
            let prompt = format!("Subtask {}/{}: {}", i + 1, subtasks.len(), subtask);
            messages.push(crate::schema::Message::user(prompt));

            let options = crate::components::model::GenerateOptions::default();

            let response = self
                .model
                .generate(messages.clone(), options)
                .await
                .map_err(|e| AgentError::Agent(e.to_string()))?;

            messages.push(response.clone());

            if let crate::schema::Content::Text(text) = response.content {
                results.push(format!("Subtask {}: {}", i + 1, text));
            }
        }

        Ok(results.join("\n\n"))
    }

    async fn synthesize(
        &self,
        results: &str,
        original_task: &str,
    ) -> Result<crate::schema::Message, AgentError> {
        let prompt = format!(
            "Original task: {}\n\nResults:\n{}\n\nProvide a final synthesized answer.",
            original_task, results
        );

        let message = crate::schema::Message::user(prompt);
        let options = crate::components::model::GenerateOptions::default();

        self.model
            .generate(vec![message], options)
            .await
            .map_err(|e| AgentError::Agent(e.to_string()))
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for DeepAgent<M> {
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
            .map(|m| match &m.content {
                crate::schema::Content::Text(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        if task.is_empty() {
            return Err(AgentError::InvalidInput("No task provided".to_string()));
        }

        let subtasks = self.decompose_task(&task).await?;

        let mut messages = input.messages.clone();
        let results = self.execute_subtasks(subtasks, &mut messages).await?;

        let final_response = self.synthesize(&results, &task).await?;

        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(final_response),
                action: None,
            },
            error: None,
        })
    }

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: crate::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        let phase: DeepPhase = serde_json::from_slice(&checkpoint)
            .map_err(|e| AgentError::Agent(format!("Invalid checkpoint: {e}")))?;

        match phase {
            DeepPhase::Decompose => {
                let task = info.user_input;
                let subtasks = self.decompose_task(&task).await?;
                let mut messages = vec![];
                let results = self.execute_subtasks(subtasks, &mut messages).await?;
                let final_response = self.synthesize(&results, &task).await?;
                Ok(AgentEvent {
                    agent_name: self.name.clone(),
                    output: AgentOutput {
                        message: Some(final_response),
                        action: None,
                    },
                    error: None,
                })
            }
            DeepPhase::Execute {
                index,
                subtasks,
                mut results,
                mut messages,
            } => {
                let task = info.user_input;
                messages.push(crate::schema::Message::user(task.clone()));
                for i in index..subtasks.len() {
                    let prompt =
                        format!("Subtask {}/{}: {}", i + 1, subtasks.len(), subtasks[i]);
                    messages.push(crate::schema::Message::user(prompt));
                    let options = crate::components::model::GenerateOptions::default();
                    let response = self
                        .model
                        .generate(messages.clone(), options)
                        .await
                        .map_err(|e| AgentError::Agent(e.to_string()))?;
                    messages.push(response.clone());
                    if let crate::schema::Content::Text(text) = response.content {
                        results.push(format!("Subtask {}: {}", i + 1, text));
                    }
                }
                let all_results = results.join("\n\n");
                let final_response = self.synthesize(&all_results, &task).await?;
                Ok(AgentEvent {
                    agent_name: self.name.clone(),
                    output: AgentOutput {
                        message: Some(final_response),
                        action: None,
                    },
                    error: None,
                })
            }
            DeepPhase::Synthesize { results, task } => {
                let final_response = self.synthesize(&results, &task).await?;
                Ok(AgentEvent {
                    agent_name: self.name.clone(),
                    output: AgentOutput {
                        message: Some(final_response),
                        action: None,
                    },
                    error: None,
                })
            }
        }
    }
}

/// Legacy LLM-based router. For new code, prefer
/// [`crate::adk::multiagent::RouterSupervisor`], which uses structured
/// tool calls for reliable multi-turn routing and supports nesting.
pub struct Supervisor<M: BaseChatModel> {
    name: String,
    model: Arc<M>,
    sub_agents: Vec<(String, Box<dyn Agent>)>,
}

impl<M: BaseChatModel + 'static> Supervisor<M> {
    pub fn new(name: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            model,
            sub_agents: Vec::new(),
        }
    }

    pub fn with_agents(mut self, agents: Vec<(String, Box<dyn Agent>)>) -> Self {
        self.sub_agents = agents;
        self
    }

    pub fn add_agent(mut self, name: impl Into<String>, agent: Box<dyn Agent>) -> Self {
        self.sub_agents.push((name.into(), agent));
        self
    }

    async fn select_agent(&self, task: &str) -> Result<String, AgentError> {
        let agent_descriptions: Vec<String> = self
            .sub_agents
            .iter()
            .map(|(name, agent)| format!("- {}: {}", name, agent.description()))
            .collect();

        let prompt = format!(
            "Select the most appropriate agent for this task:\n\nTask: {}\n\nAvailable agents:\n{}\n\nRespond with only the agent name.",
            task,
            agent_descriptions.join("\n")
        );

        let message = crate::schema::Message::user(prompt);
        let options = crate::components::model::GenerateOptions::default();

        let response = self
            .model
            .generate(vec![message], options)
            .await
            .map_err(|e| AgentError::Agent(e.to_string()))?;

        let content = match response.content {
            crate::schema::Content::Text(s) => s.trim().to_string(),
            _ => return Err(AgentError::Agent("Invalid response".to_string())),
        };

        Ok(content)
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static> Agent for Supervisor<M> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        format!(
            "Supervisor agent coordinating {} sub-agents",
            self.sub_agents.len()
        )
    }

    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        let task = input
            .messages
            .last()
            .map(|m| match &m.content {
                crate::schema::Content::Text(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        if task.is_empty() {
            return Err(AgentError::InvalidInput("No task provided".to_string()));
        }

        let selected_name = self.select_agent(&task).await?;

        for (name, agent) in &self.sub_agents {
            if name == &selected_name {
                return agent.run(input).await;
            }
        }

        Err(AgentError::Agent(format!(
            "Agent {} not found",
            selected_name
        )))
    }

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: crate::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        #[derive(serde::Deserialize)]
        struct SupervisorCheckpoint {
            agent_name: String,
            inner_checkpoint: Vec<u8>,
        }
        let state: SupervisorCheckpoint = serde_json::from_slice(&checkpoint)
            .map_err(|e| AgentError::Agent(format!("Invalid checkpoint: {e}")))?;
        for (name, agent) in &self.sub_agents {
            if name == &state.agent_name {
                return agent.resume(state.inner_checkpoint, info).await;
            }
        }
        Err(AgentError::Agent(format!(
            "Agent {} not found",
            state.agent_name
        )))
    }
}
