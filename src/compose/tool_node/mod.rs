use crate::components::tool::{BaseTool, InvokableTool};
use crate::compose::runnable::{Context, Runnable, RunnableError, RunnableFuture};
use crate::compose::StreamReader;
use crate::schema::{Message, RoleType, ToolInfo, ToolResult};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ToolsNodeError {
    #[error("tool {0} not found in toolsNode")]
    ToolNotFound(String),
    #[error("invalid input: expected Assistant role, got {0:?}")]
    InvalidRole(RoleType),
    #[error("no tool calls found in input message")]
    NoToolCalls,
    #[error("tool {0} is not invokable")]
    NotInvokable(String),
    #[error("tool execution error [{name}]: {error}")]
    Execution { name: String, error: String },
    #[error("failed to execute tool arguments handler for {name}: {error}")]
    ArgumentsHandler { name: String, error: String },
}

pub struct ToolInput {
    pub name: String,
    pub arguments: String,
    pub call_id: String,
}

pub struct ToolOutput {
    pub result: String,
}

pub struct StreamToolOutput {
    pub result: StreamReader<String>,
}

pub struct EnhancedInvokableToolOutput {
    pub result: ToolResult,
}

pub struct EnhancedStreamToolOutput {
    pub result: StreamReader<ToolResult>,
}

pub type InvokableEndpoint =
    Arc<dyn Fn(ToolInput) -> Result<ToolOutput, ToolsNodeError> + Send + Sync + 'static>;
pub type StreamEndpoint =
    Arc<dyn Fn(ToolInput) -> Result<StreamToolOutput, ToolsNodeError> + Send + Sync + 'static>;
pub type EnhancedInvokableEndpoint = Arc<
    dyn Fn(ToolInput) -> Result<EnhancedInvokableToolOutput, ToolsNodeError>
        + Send
        + Sync
        + 'static,
>;
pub type EnhancedStreamEndpoint = Arc<
    dyn Fn(ToolInput) -> Result<EnhancedStreamToolOutput, ToolsNodeError> + Send + Sync + 'static,
>;

struct ToolsTuple {
    indexes: HashMap<String, usize>,
    tool_infos: Vec<ToolInfo>,
    invokable_endpoints: Vec<Option<InvokableEndpoint>>,
}

fn wrap_invokable_tool<T: InvokableTool + 'static>(tool: Arc<T>) -> InvokableEndpoint {
    Arc::new(move |input: ToolInput| {
        let result = futures::executor::block_on(tool.invoke(input.arguments));
        match result {
            Ok(s) => Ok(ToolOutput { result: s }),
            Err(e) => Err(ToolsNodeError::Execution {
                name: input.name,
                error: e.to_string(),
            }),
        }
    })
}

fn conv_tools<T: BaseTool + InvokableTool + 'static>(
    tools: &[Arc<T>],
) -> Result<ToolsTuple, ToolsNodeError> {
    let n = tools.len();
    let mut tuple = ToolsTuple {
        indexes: HashMap::with_capacity(n),
        tool_infos: Vec::with_capacity(n),
        invokable_endpoints: vec![None; n],
    };

    for (idx, tool) in tools.iter().enumerate() {
        let info = futures::executor::block_on(tool.info())
            .map_err(|e| ToolsNodeError::NotInvokable(format!("failed to get tool info: {}", e)))?;
        let tool_name = info.name.clone();
        tuple.tool_infos.push(info);
        tuple.invokable_endpoints[idx] = Some(wrap_invokable_tool(tool.clone()));
        tuple.indexes.insert(tool_name, idx);
    }

    Ok(tuple)
}

pub struct ToolsNodeConfig<T: BaseTool + InvokableTool + 'static> {
    pub tools: Vec<Arc<T>>,
    pub unknown_tool_handler:
        Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>>,
    pub execute_sequentially: bool,
    pub tool_arguments_handler:
        Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>>,
}

impl<T: BaseTool + InvokableTool + 'static> Default for ToolsNodeConfig<T> {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            unknown_tool_handler: None,
            execute_sequentially: false,
            tool_arguments_handler: None,
        }
    }
}

pub struct ToolsNode {
    tuple: ToolsTuple,
    unknown_tool_handler:
        Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>>,
    execute_sequentially: bool,
    tool_arguments_handler:
        Option<Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>>,
}

impl<T: BaseTool + InvokableTool + 'static> ToolsNodeConfig<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ToolsNode {
    pub fn new<T: BaseTool + InvokableTool + 'static>(
        config: ToolsNodeConfig<T>,
    ) -> Result<Self, ToolsNodeError> {
        let tuple = conv_tools(&config.tools)?;
        Ok(Self {
            tuple,
            unknown_tool_handler: config.unknown_tool_handler,
            execute_sequentially: config.execute_sequentially,
            tool_arguments_handler: config.tool_arguments_handler,
        })
    }
}

impl Runnable<Message, Vec<Message>> for ToolsNode {
    fn invoke(&self, _ctx: Context, input: Message) -> RunnableFuture<Vec<Message>> {
        let execute_seq = self.execute_sequentially;
        let indexes: HashMap<_, _> = self.tuple.indexes.clone();
        let inv_endpoints: Vec<_> = self.tuple.invokable_endpoints.clone();
        let unknown_handler = self.unknown_tool_handler.clone();
        let args_handler = self.tool_arguments_handler.clone();

        Box::pin(async move {
            if input.role != RoleType::Assistant {
                return Err(RunnableError::Invoke(format!(
                    "expected Assistant role, got {:?}",
                    input.role
                )));
            }

            let tool_calls = input.tool_calls.as_ref().ok_or_else(|| {
                RunnableError::InvalidInput("no tool_calls in message".to_string())
            })?;

            if tool_calls.is_empty() {
                return Err(RunnableError::InvalidInput(
                    "no tool calls found".to_string(),
                ));
            }

            let n = tool_calls.len();
            let mut outputs: Vec<Option<String>> = vec![None; n];
            let mut errors: Vec<Option<String>> = vec![None; n];

            if execute_seq {
                for (i, tc) in tool_calls.iter().enumerate() {
                    let (out, err) = run_single_tool(
                        &tc.function.name,
                        &tc.id,
                        &tc.function.arguments,
                        &indexes,
                        &inv_endpoints,
                        unknown_handler.as_ref(),
                        args_handler.as_ref(),
                    );
                    outputs[i] = out;
                    errors[i] = err;
                }
            } else {
                let mut handles = Vec::with_capacity(n);
                for tc in tool_calls {
                    let name = tc.function.name.clone();
                    let call_id = tc.id.clone();
                    let args = tc.function.arguments.clone();
                    let idx_clone = indexes.clone();
                    let eps_clone: Vec<_> = inv_endpoints.iter().cloned().collect();
                    let unknown_clone = unknown_handler.clone();
                    let args_handler_clone = args_handler.clone();
                    handles.push(tokio::task::spawn_blocking(move || {
                        run_single_tool_owned(
                            name,
                            call_id,
                            args,
                            idx_clone,
                            eps_clone,
                            unknown_clone,
                            args_handler_clone,
                        )
                    }));
                }

                for (i, h) in handles.into_iter().enumerate() {
                    let (out, err) = h.await.unwrap_or((None, Some("panic".to_string())));
                    outputs[i] = out;
                    errors[i] = err;
                }
            }

            let mut messages = Vec::with_capacity(n);
            for (i, tc) in tool_calls.iter().enumerate() {
                if let Some(ref e) = errors[i] {
                    return Err(RunnableError::Invoke(format!(
                        "tool {} error: {}",
                        tc.function.name, e
                    )));
                }
                let mut msg = Message::tool(outputs[i].clone().unwrap_or_default(), tc.id.clone());
                msg.name = Some(tc.function.name.clone());
                messages.push(msg);
            }

            Ok(messages)
        })
    }

    fn name(&self) -> String {
        "tools_node".to_string()
    }
}

fn run_single_tool_owned(
    name: String,
    call_id: String,
    args: String,
    indexes: HashMap<String, usize>,
    inv_endpoints: Vec<Option<InvokableEndpoint>>,
    unknown_handler: Option<
        Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>,
    >,
    args_handler: Option<
        Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>,
    >,
) -> (Option<String>, Option<String>) {
    let processed_args = match args_handler {
        Some(h) => match h(name.clone(), args) {
            Ok(a) => a,
            Err(e) => return (None, Some(e)),
        },
        None => args,
    };

    if let Some(&idx) = indexes.get(&name) {
        if let Some(ep) = &inv_endpoints[idx] {
            let input = ToolInput {
                name: name.clone(),
                arguments: processed_args,
                call_id,
            };
            match ep(input) {
                Ok(out) => (Some(out.result), None),
                Err(e) => (None, Some(e.to_string())),
            }
        } else {
            (
                None,
                Some(format!("tool {} has no invokable endpoint", name)),
            )
        }
    } else if let Some(h) = unknown_handler {
        match h(name, processed_args) {
            Ok(result) => (Some(result), None),
            Err(e) => (None, Some(e)),
        }
    } else {
        (None, Some(format!("tool {} not found", name)))
    }
}

fn run_single_tool(
    name: &str,
    call_id: &str,
    args: &str,
    indexes: &HashMap<String, usize>,
    inv_endpoints: &[Option<InvokableEndpoint>],
    unknown_handler: Option<
        &Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>,
    >,
    args_handler: Option<
        &Arc<dyn Fn(String, String) -> Result<String, String> + Send + Sync + 'static>,
    >,
) -> (Option<String>, Option<String>) {
    let processed_args = match args_handler {
        Some(h) => match h(name.to_string(), args.to_string()) {
            Ok(a) => a,
            Err(e) => return (None, Some(e)),
        },
        None => args.to_string(),
    };

    if let Some(&idx) = indexes.get(name) {
        if let Some(ref ep) = inv_endpoints[idx] {
            let input = ToolInput {
                name: name.to_string(),
                arguments: processed_args,
                call_id: call_id.to_string(),
            };
            match ep(input) {
                Ok(out) => (Some(out.result), None),
                Err(e) => (None, Some(e.to_string())),
            }
        } else {
            (
                None,
                Some(format!("tool {} has no invokable endpoint", name)),
            )
        }
    } else if let Some(h) = unknown_handler {
        match h(name.to_string(), processed_args) {
            Ok(result) => (Some(result), None),
            Err(e) => (None, Some(e)),
        }
    } else {
        (None, Some(format!("tool {} not found", name)))
    }
}
