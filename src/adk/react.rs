use async_trait::async_trait;
use std::sync::Arc;
use crate::schema::{Message, RoleType, Content, ToolCall, ToolInfo};
use crate::components::model::{BaseChatModel, GenerateOptions};
use crate::components::tool::InvokableTool;
use crate::adk::agent::{Agent, AgentInput, AgentEvent, AgentError, AgentOutput};
use crate::skills::{SkillManager, SkillContext, SkillError};

pub struct ChatModelAgent<M: BaseChatModel, T: InvokableTool> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<T>>,
    skills: Arc<tokio::sync::RwLock<SkillManager>>,
    max_iterations: usize,
    temperature: Option<f32>,
}

impl<M: BaseChatModel + 'static, T: InvokableTool + 'static> ChatModelAgent<M, T> {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        model: Arc<M>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            model,
            tools: Vec::new(),
            skills: Arc::new(tokio::sync::RwLock::new(SkillManager::new())),
            max_iterations: 10,
            temperature: None,
        }
    }
    
    pub fn with_tools(mut self, tools: Vec<Arc<T>>) -> Self {
        self.tools = tools;
        self
    }
    
    pub fn with_skill_manager(mut self, manager: SkillManager) -> Self {
        self.skills = Arc::new(tokio::sync::RwLock::new(manager));
        self
    }
    
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
    
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }
    
    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<String, AgentError> {
        let tool_name = &tool_call.function.name;
        
        for tool in &self.tools {
            let info = tool.info().await.map_err(|e| AgentError::ToolError(e.to_string()))?;
            if info.name == *tool_name {
                let result = tool
                    .invoke(tool_call.function.arguments.clone())
                    .await
                    .map_err(|e| AgentError::ToolError(e.to_string()))?;
                return Ok(result);
            }
        }
        
        Err(AgentError::ToolError(format!("Tool {} not found", tool_name)))
    }
    
    async fn get_tool_infos(&self) -> Vec<ToolInfo> {
        let mut infos = Vec::new();
        for tool in &self.tools {
            if let Ok(info) = tool.info().await {
                infos.push(info);
            }
        }
        infos
    }
    
    pub async fn run_loop(&self, messages: Vec<Message>) -> Result<AgentEvent, AgentError> {
        let mut iterations = 0;
        let mut current_messages = messages;
        
        // Check if input matches any skill
        if let Some(first_msg) = current_messages.first() {
            let input_text = match &first_msg.content {
                Content::Text(text) => text.clone(),
                _ => String::new(),
            };
            
            if !input_text.is_empty() {
                let skills = self.skills.read().await;
                match skills.execute_matched(
                    &SkillContext::new().with_messages(current_messages.clone()),
                    &input_text
                ).await {
                    Ok(output) => {
                        let response_msg = Message {
                            role: RoleType::Assistant,
                            content: Content::Text(output),
                            name: None,
                            tool_call_id: None,
                            tool_calls: None,
                        };
                        return Ok(AgentEvent {
                            agent_name: self.name.clone(),
                            output: AgentOutput {
                                message: Some(response_msg),
                                action: None,
                            },
                            error: None,
                        });
                    }
                    Err(SkillError::NotFound(_)) => {
                        // No skill matched, continue with normal flow
                    }
                    Err(e) => {
                        return Err(AgentError::Agent(format!("Skill error: {}", e)));
                    }
                }
            }
        }
        
        loop {
            if iterations >= self.max_iterations {
                return Err(AgentError::MaxIterationsExceeded);
            }
            iterations += 1;
            
            let options = GenerateOptions {
                temperature: self.temperature,
                tools: Some(self.get_tool_infos().await),
                ..Default::default()
            };
            
            let response = self.model
                .generate(current_messages.clone(), options)
                .await
                .map_err(|e| AgentError::Agent(e.to_string()))?;
            
            current_messages.push(response.clone());
            
            if let Some(tool_calls) = response.tool_calls {
                for tool_call in tool_calls {
                    let tool_result = self.execute_tool(&tool_call).await?;
                    let tool_message = Message {
                        role: RoleType::Tool,
                        content: Content::Text(tool_result),
                        name: None,
                        tool_call_id: Some(tool_call.id.clone()),
                        tool_calls: None,
                    };
                    current_messages.push(tool_message);
                }
            } else {
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
    }
}

#[async_trait]
impl<M: BaseChatModel + Send + Sync + 'static, T: InvokableTool + Send + Sync + 'static> Agent for ChatModelAgent<M, T> {
    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn description(&self) -> String {
        self.description.clone()
    }
    
    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        self.run_loop(input.messages).await
    }
    
    async fn resume(&self, _checkpoint: Vec<u8>, _info: crate::adk::agent::ResumeInfo) -> Result<AgentEvent, AgentError> {
        Err(AgentError::Agent("Resume not implemented".to_string()))
    }
}

pub struct SimpleTool {
    name: String,
    description: String,
    handler: Arc<dyn Fn(String) -> Result<String, String> + Send + Sync>,
}

impl SimpleTool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        handler: impl Fn(String) -> Result<String, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            handler: Arc::new(handler),
        }
    }
}

#[async_trait]
impl InvokableTool for SimpleTool {
    async fn invoke(&self, arguments: String) -> Result<String, crate::components::tool::ToolError> {
        (self.handler)(arguments).map_err(|e| crate::components::tool::ToolError::Execution(e))
    }
}

impl crate::components::tool::BaseTool for SimpleTool {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, crate::components::tool::ToolError>> + Send {
        async move {
            Ok(ToolInfo {
                name: self.name.clone(),
                description: self.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            })
        }
    }
}
