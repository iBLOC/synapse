use crate::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, AgentStream, AgentStreamEvent};
use crate::components::model::{BaseChatModel, FinishReason, GenerateOptions};
use crate::components::tool::{BaseTool, EnhancedInvokableTool, InvokableTool, ToolError};
use crate::schema::{Content, Message, RoleType, ToolCall, ToolInfo, ToolResult};
use crate::skills::{SkillContext, SkillError, SkillManager};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Instant;

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
    system_prompt: Option<String>,
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
            system_prompt: None,
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

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
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

    pub async fn run_loop_streaming(
        &self,
        messages: Vec<Message>,
    ) -> Result<AgentStream, AgentError> {
        let (writer, reader) = crate::stream::StreamReader::<AgentStreamEvent>::new(128);
        let cancel = reader.cancel_handle();

        let mut current_messages = messages;
        if let Some(prompt) = &self.system_prompt {
            current_messages.insert(0, Message::system(prompt.clone()));
        }

        let tool_infos = self.get_tool_infos().await;
        let agent_name = self.name.clone();
        let model = self.model.clone();
        let tools = self.tools.clone();
        let tool_middleware = self.tool_middleware.clone();
        let max_iterations = self.max_iterations;
        let options = GenerateOptions {
            temperature: self.temperature,
            tools: Some(tool_infos),
            ..Default::default()
        };

        tokio::spawn(async move {
            let mut iterations: usize = 0;
            loop {
                if cancel.is_cancelled() {
                    return;
                }
                if iterations >= max_iterations {
                    let _ = writer
                        .send(AgentStreamEvent::Error {
                            agent: agent_name.clone(),
                            error: AgentError::MaxIterationsExceeded,
                        })
                        .await;
                    return;
                }
                iterations += 1;
                let _ = writer
                    .send(AgentStreamEvent::StepTransition {
                        agent: agent_name.clone(),
                        from: "iter_end".into(),
                        to: "model_call".into(),
                        step: iterations,
                    })
                    .await;

                let model_stream_res = model
                    .stream_deltas(current_messages.clone(), options.clone())
                    .await;
                let mut stream = match model_stream_res {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = writer
                            .send(AgentStreamEvent::Error {
                                agent: agent_name.clone(),
                                error: AgentError::Agent(e.to_string()),
                            })
                            .await;
                        return;
                    }
                };

                let mut msg = Message::assistant("");
                while let Some(delta_res) = stream.next().await {
                    if cancel.is_cancelled() {
                        return;
                    }
                    if !delta_res.content_delta.is_empty() {
                        let _ = writer
                            .send(AgentStreamEvent::TokenDelta {
                                agent: agent_name.clone(),
                                delta: delta_res.content_delta.clone(),
                            })
                            .await;
                        if let Content::Text(ref mut text) = msg.content {
                            text.push_str(&delta_res.content_delta);
                        }
                    }
                    for tc_d in &delta_res.tool_call_deltas {
                        merge_tool_call_delta(&mut msg, tc_d.clone());
                    }
                    if matches!(delta_res.finish_reason, Some(FinishReason::Error)) {
                        let _ = writer
                            .send(AgentStreamEvent::Error {
                                agent: agent_name.clone(),
                                error: AgentError::Agent("stream error".into()),
                            })
                            .await;
                        return;
                    }
                }
                current_messages.push(msg.clone());

                if let Some(calls) = msg.tool_calls.clone() {
                    if iterations >= max_iterations {
                        let _ = writer
                            .send(AgentStreamEvent::Error {
                                agent: agent_name.clone(),
                                error: AgentError::MaxIterationsExceeded,
                            })
                            .await;
                        return;
                    }
                    for call in calls {
                        let _ = writer
                            .send(AgentStreamEvent::ToolStart {
                                agent: agent_name.clone(),
                                name: call.function.name.clone(),
                                args: call.function.arguments.clone(),
                            })
                            .await;
                        let start = Instant::now();
                        let result = execute_tool(
                            &tools,
                            &tool_middleware,
                            &call,
                        )
                        .await;
                        let latency_ms = start.elapsed().as_millis() as u64;
                        match result {
                            Ok(result_text) => {
                                let _ = writer
                                    .send(AgentStreamEvent::ToolEnd {
                                        agent: agent_name.clone(),
                                        name: call.function.name.clone(),
                                        result: result_text.clone(),
                                        latency_ms,
                                    })
                                    .await;
                                current_messages.push(Message {
                                    role: RoleType::Tool,
                                    content: Content::Text(result_text),
                                    name: None,
                                    tool_call_id: Some(call.id.clone()),
                                    tool_calls: None,
                                });
                            }
                            Err(e) => {
                                let _ = writer
                                    .send(AgentStreamEvent::Error {
                                        agent: agent_name.clone(),
                                        error: e,
                                    })
                                    .await;
                                return;
                            }
                        }
                    }
                } else {
                    let _ = writer
                        .send(AgentStreamEvent::Final {
                            agent: agent_name.clone(),
                            message: msg,
                        })
                        .await;
                    return;
                }
            }
        });

        Ok(reader)
    }

    pub async fn run_loop(&self, messages: Vec<Message>) -> Result<AgentEvent, AgentError> {
        let mut iterations = 0;
        let mut current_messages = messages;

        if let Some(prompt) = &self.system_prompt {
            current_messages.insert(0, Message::system(prompt.clone()));
        }

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

    async fn stream_run(&self, input: AgentInput) -> AgentStream {
        match self.run_loop_streaming(input.messages).await {
            Ok(stream) => stream,
            Err(e) => {
                let (writer, reader) =
                    crate::stream::StreamReader::<AgentStreamEvent>::new(8);
                let name = self.name.clone();
                tokio::spawn(async move {
                    let _ = writer
                        .send(AgentStreamEvent::Error {
                            agent: name,
                            error: e,
                        })
                        .await;
                });
                reader
            }
        }
    }

    async fn resume(
        &self,
        checkpoint: Vec<u8>,
        info: crate::adk::agent::ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        let mut messages: Vec<Message> = serde_json::from_slice(&checkpoint)
            .map_err(|e| AgentError::Agent(format!("Invalid checkpoint: {e}")))?;
        messages.push(Message::user(info.user_input));
        self.run_loop(messages).await
    }
}

fn merge_tool_call_delta(msg: &mut Message, tc_d: crate::components::model::ToolCallDelta) {
    use crate::schema::{FunctionCall, ToolCall};
    let calls = msg.tool_calls.get_or_insert_with(Vec::new);
    while calls.len() <= tc_d.index {
        calls.push(ToolCall {
            id: String::new(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: String::new(),
                arguments: String::new(),
            },
        });
    }
    let slot = &mut calls[tc_d.index];
    if let Some(id) = &tc_d.id {
        slot.id = id.clone();
    }
    if let Some(name) = &tc_d.name {
        slot.function.name = name.clone();
    }
    slot.function.arguments.push_str(&tc_d.arguments_delta);
}

async fn execute_tool<T: InvokableTool + Send + Sync + 'static>(
    tools: &[Arc<T>],
    tool_middleware: &[Arc<dyn ToolMiddleware>],
    tool_call: &ToolCall,
) -> Result<String, AgentError> {
    let tool_name = &tool_call.function.name;
    let args = &tool_call.function.arguments;
    for tool in tools {
        let info = tool
            .info()
            .await
            .map_err(|e| AgentError::ToolError(e.to_string()))?;
        if info.name == *tool_name {
            for mw in tool_middleware {
                if let Err(e) = mw.before_tool(tool_name, args) {
                    tracing::warn!("tool middleware before_tool failed: {}", e);
                }
            }
            let result = tool
                .invoke(tool_call.function.arguments.clone())
                .await
                .map_err(|e| AgentError::ToolError(e.to_string()))?;
            for mw in tool_middleware.iter().rev() {
                if let Err(e) = mw.after_tool(tool_name, &result) {
                    tracing::warn!("tool middleware after_tool failed: {}", e);
                }
            }
            return Ok(result);
        }
    }
    Err(AgentError::ToolError(format!(
        "Tool {} not found",
        tool_name
    )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::model::{
        BaseChatModel, FinishReason, GenerateOptions, MessageDelta, ModelError, ModelResult,
        StreamReader,
    };
    use crate::schema::{Content, Message, RoleType};
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// In-memory mock that satisfies [`BaseChatModel`].
    ///
    /// `ScriptedChatModel::new` returns the same scripted deltas once,
    /// then sends EOF. `ScriptedChatModel::looping` re-emits the same
    /// scripted deltas on every call (used to drive max-iterations tests).
    struct ScriptedChatModel {
        deltas: Vec<MessageDelta>,
        loop_forever: bool,
    }

    impl ScriptedChatModel {
        fn new(deltas: Vec<MessageDelta>) -> Self {
            Self {
                deltas,
                loop_forever: false,
            }
        }

        fn looping(deltas: Vec<MessageDelta>) -> Self {
            Self {
                deltas,
                loop_forever: true,
            }
        }
    }

    #[async_trait]
    impl BaseChatModel for ScriptedChatModel {
        async fn generate(
            &self,
            _input: Vec<Message>,
            _options: GenerateOptions,
        ) -> ModelResult<Message> {
            Ok(Message::assistant("(scripted: no final answer)"))
        }

        async fn stream(
            &self,
            _input: Vec<Message>,
            _options: GenerateOptions,
        ) -> ModelResult<StreamReader> {
            Err(ModelError::Api("legacy stream not implemented".into()))
        }

        async fn stream_deltas(
            &self,
            _input: Vec<Message>,
            _options: GenerateOptions,
        ) -> ModelResult<crate::stream::StreamReader<MessageDelta>> {
            let (writer, reader) = crate::stream::StreamReader::<MessageDelta>::new(8);
            let script = self.deltas.clone();
            let loop_forever = self.loop_forever;
            tokio::spawn(async move {
                loop {
                    for d in &script {
                        if !writer.send(d.clone()).await {
                            return;
                        }
                    }
                    if !loop_forever {
                        return;
                    }
                }
            });
            Ok(reader)
        }
    }

    fn assistant_text_delta(s: &str) -> MessageDelta {
        MessageDelta {
            content_delta: s.to_string(),
            tool_call_deltas: vec![],
            finish_reason: None,
            usage: None,
        }
    }

    fn terminal_delta(reason: FinishReason) -> MessageDelta {
        MessageDelta {
            content_delta: String::new(),
            tool_call_deltas: vec![],
            finish_reason: Some(reason),
            usage: None,
        }
    }

    fn tool_call_delta(name: &str, args: &str) -> MessageDelta {
        use crate::components::model::ToolCallDelta;
        MessageDelta {
            content_delta: String::new(),
            tool_call_deltas: vec![ToolCallDelta {
                index: 0,
                id: Some("call_1".to_string()),
                name: Some(name.to_string()),
                arguments_delta: args.to_string(),
            }],
            finish_reason: None,
            usage: None,
        }
    }

    #[tokio::test]
    async fn run_loop_streaming_emits_text_then_final() {
        // A simple scripted model: emit "hi" then a Stop terminal delta.
        let model = Arc::new(ScriptedChatModel::new(vec![
            assistant_text_delta("hi "),
            assistant_text_delta("there"),
            terminal_delta(FinishReason::Stop),
        ]));
        let agent = ChatModelAgent::<ScriptedChatModel, SimpleTool>::new(
            "tester",
            "tests ChatModelAgent streaming",
            model,
        );

        let mut stream = agent
            .run_loop_streaming(vec![Message::user("ping")])
            .await
            .expect("stream_run ok");

        let mut token_count = 0;
        let mut final_seen = false;
        while let Some(event) = stream.next().await {
            match event {
                AgentStreamEvent::TokenDelta { agent, delta } => {
                    assert_eq!(agent, "tester");
                    token_count += 1;
                }
                AgentStreamEvent::Final { agent, message } => {
                    assert_eq!(agent, "tester");
                    final_seen = true;
                    match message.content {
                        Content::Text(t) => {
                            assert!(t.contains("hi ") || t.contains("there"));
                        }
                        _ => panic!("Final message should be Text"),
                    }
                }
                AgentStreamEvent::Error { agent, error } => {
                    panic!("unexpected error event from {}: {}", agent, error);
                }
                _ => {
                    // StepTransition, etc. — ignored; we only assert on the
                    // text + final pair to keep the test focused.
                }
            }
        }
        assert_eq!(token_count, 2, "expected 2 token deltas");
        assert!(final_seen, "expected a Final event");
    }

    #[tokio::test]
    async fn run_loop_streaming_respects_max_iterations() {
        // Run with max_iterations(1) so the second iteration triggers
        // the MaxIterationsExceeded error. The model returns a text
        // delta without a tool call, so the first iteration produces
        // a Final — and the loop should not advance. To exercise the
        // max-iter code path, we set max_iterations(0) which makes
        // the first check at the top of the loop fire immediately.
        let model = Arc::new(ScriptedChatModel::new(vec![
            assistant_text_delta("ignored"),
            terminal_delta(FinishReason::Stop),
        ]));
        let agent = ChatModelAgent::<ScriptedChatModel, SimpleTool>::new(
            "zero-iter",
            "max_iterations(0) trips before any model call",
            model,
        )
        .max_iterations(0);

        let mut stream = agent
            .run_loop_streaming(vec![Message::user("ping")])
            .await
            .expect("stream_run ok");

        let mut saw_max_iter = false;
        while let Some(event) = stream.next().await {
            if let AgentStreamEvent::Error { error, .. } = event {
                if matches!(error, crate::adk::agent::AgentError::MaxIterationsExceeded) {
                    saw_max_iter = true;
                }
            }
        }
        assert!(
            saw_max_iter,
            "max_iterations(0) should emit MaxIterationsExceeded before any model call"
        );
    }

    #[tokio::test]
    async fn run_loop_streaming_final_message_accumulates_deltas() {
        // Verify that the Final message contains the concatenated text of
        // every token delta, in order, with no text dropped.
        let model = Arc::new(ScriptedChatModel::new(vec![
            assistant_text_delta("a"),
            assistant_text_delta("b"),
            assistant_text_delta("c"),
            terminal_delta(FinishReason::Stop),
        ]));
        let agent = ChatModelAgent::<ScriptedChatModel, SimpleTool>::new(
            "accum",
            "tests accumulation",
            model,
        );
        let mut stream = agent
            .run_loop_streaming(vec![Message::user("ping")])
            .await
            .expect("stream_run ok");

        let mut final_text = None;
        while let Some(event) = stream.next().await {
            if let AgentStreamEvent::Final { message, .. } = event {
                final_text = match message.content {
                    Content::Text(t) => Some(t),
                    _ => None,
                };
            }
        }
        let t = final_text.expect("Final event should have been emitted");
        assert!(
            t.contains('a') && t.contains('b') && t.contains('c'),
            "Final text should accumulate a, b, c — got {:?}",
            t
        );
    }
}
