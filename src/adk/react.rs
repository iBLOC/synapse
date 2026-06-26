use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput};
use crate::components::model::{BaseChatModel, GenerateOptions};
use crate::components::tool::{BaseTool, EnhancedInvokableTool, InvokableTool, ToolError};
use crate::schema::{Content, Message, RoleType, ToolArgument, ToolCall, ToolInfo, ToolResult};
use crate::skills::{SkillContext, SkillError, SkillManager};
use async_trait::async_trait;
use std::sync::Arc;

/// ToolMiddleware provides pre/post hooks for tool execution.
/// This allows tools to be wrapped with logging, tracing, error handling, etc.
pub trait ToolMiddleware: Send + Sync {
    fn before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError>;
    fn after_tool(&self, name: &str, result: &str) -> Result<(), ToolError>;
}

pub struct ToolMiddlewareChain {
    middlewares: Vec<Arc<dyn ToolMiddleware>>,
}

impl ToolMiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add(mut self, mw: Arc<dyn ToolMiddleware>) -> Self {
        self.middlewares.push(mw);
        self
    }

    pub fn before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError> {
        for mw in &self.middlewares {
            mw.before_tool(name, arguments)?;
        }
        Ok(())
    }

    pub fn after_tool(&self, name: &str, result: &str) -> Result<(), ToolError> {
        for mw in self.middlewares.iter().rev() {
            mw.after_tool(name, result)?;
        }
        Ok(())
    }
}

impl Default for ToolMiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ChatModelAgent<M: BaseChatModel, T: InvokableTool> {
    name: String,
    description: String,
    model: Arc<M>,
    tools: Vec<Arc<T>>,
    skills: Arc<tokio::sync::RwLock<SkillManager>>,
    max_iterations: usize,
    temperature: Option<f32>,
    tool_middleware: Vec<Arc<dyn ToolMiddleware>>,
}

impl<M: BaseChatModel + 'static, T: InvokableTool + 'static> ChatModelAgent<M, T> {
    pub fn new(name: impl Into<String>, description: impl Into<String>, model: Arc<M>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            model,
            tools: Vec::new(),
            skills: Arc::new(tokio::sync::RwLock::new(SkillManager::new())),
            max_iterations: 10,
            temperature: None,
            tool_middleware: Vec::new(),
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

    pub fn with_tool_middleware(mut self, middleware: Arc<dyn ToolMiddleware>) -> Self {
        self.tool_middleware.push(middleware);
        self
    }

    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<String, AgentError> {
        let tool_name = &tool_call.function.name;
        let args = &tool_call.function.arguments;

        for tool in &self.tools {
            let info = tool
                .info()
                .await
                .map_err(|e| AgentError::ToolError(e.to_string()))?;
            if info.name == *tool_name {
                if let Err(e) = self.run_before_tool(tool_name, args) {
                    tracing::warn!("tool middleware before_tool failed: {}", e);
                }
                let result = tool
                    .invoke(tool_call.function.arguments.clone())
                    .await
                    .map_err(|e| AgentError::ToolError(e.to_string()))?;
                if let Err(e) = self.run_after_tool(tool_name, &result) {
                    tracing::warn!("tool middleware after_tool failed: {}", e);
                }
                return Ok(result);
            }
        }

        Err(AgentError::ToolError(format!(
            "Tool {} not found",
            tool_name
        )))
    }

    fn run_before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError> {
        for mw in &self.tool_middleware {
            mw.before_tool(name, arguments)?;
        }
        Ok(())
    }

    fn run_after_tool(&self, name: &str, result: &str) -> Result<(), ToolError> {
        for mw in self.tool_middleware.iter().rev() {
            mw.after_tool(name, result)?;
        }
        Ok(())
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
                match skills
                    .execute_matched(
                        &SkillContext::new().with_messages(current_messages.clone()),
                        &input_text,
                    )
                    .await
                {
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

            let response = self
                .model
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
impl<M: BaseChatModel + Send + Sync + 'static, T: InvokableTool + Send + Sync + 'static> Agent
    for ChatModelAgent<M, T>
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: AgentInput) -> Result<AgentEvent, AgentError> {
        self.run_loop(input.messages).await
    }

    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: crate::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
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
    async fn invoke(
        &self,
        arguments: String,
    ) -> Result<String, crate::components::tool::ToolError> {
        (self.handler)(arguments).map_err(|e| crate::components::tool::ToolError::Execution(e))
    }
}

impl crate::components::tool::BaseTool for SimpleTool {
    fn info(
        &self,
    ) -> impl std::future::Future<Output = Result<ToolInfo, crate::components::tool::ToolError>> + Send
    {
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

/// SimpleEnhancedTool is a tool that returns structured ToolResult content.
/// Unlike SimpleTool which returns a plain string, this can return
/// text, images, audio, video, or error content.
pub struct SimpleEnhancedTool {
    name: String,
    description: String,
    handler: Arc<dyn Fn(crate::schema::ToolArgument) -> Result<ToolResult, String> + Send + Sync>,
}

impl SimpleEnhancedTool {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        handler: impl Fn(crate::schema::ToolArgument) -> Result<ToolResult, String>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            handler: Arc::new(handler),
        }
    }
}

impl crate::components::tool::BaseTool for SimpleEnhancedTool {
    fn info(
        &self,
    ) -> impl std::future::Future<Output = Result<ToolInfo, crate::components::tool::ToolError>> + Send
    {
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

#[async_trait]
impl EnhancedInvokableTool for SimpleEnhancedTool {
    async fn invoke_enhanced(
        &self,
        arguments: crate::schema::ToolArgument,
    ) -> Result<ToolResult, crate::components::tool::ToolError> {
        (self.handler)(arguments).map_err(|e| crate::components::tool::ToolError::Execution(e))
    }
}

pub struct LoggingToolMiddleware {
    pub log_level: tracing::Level,
}

impl LoggingToolMiddleware {
    pub fn new() -> Self {
        Self {
            log_level: tracing::Level::DEBUG,
        }
    }
}

impl Default for LoggingToolMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolMiddleware for LoggingToolMiddleware {
    fn before_tool(&self, name: &str, arguments: &str) -> Result<(), ToolError> {
        tracing::debug!(tool = %name, args = %arguments, "tool called");
        Ok(())
    }

    fn after_tool(&self, name: &str, result: &str) -> Result<(), ToolError> {
        tracing::debug!(tool = %name, result = %result, "tool completed");
        Ok(())
    }
}
