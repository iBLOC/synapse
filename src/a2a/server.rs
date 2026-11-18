//! A2A server wrapper — adapts Synapse agents to the a2a-rs SDK.
//!
//! This module provides:
//! - [`SynapseAgentExecutor`]: implements `a2a_server::AgentExecutor` trait
//! - [`A2AServerBuilder`]: fluent builder for hosting an agent as an A2A server

use std::sync::Arc;

use a2a_server::{AgentExecutor, DefaultRequestHandler, InMemoryTaskStore};
use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::mpsc;

use crate::adk::agent::{Agent, AgentInput, AgentStreamEvent};
use crate::a2a::tool_event::tool_event_metadata;
use crate::a2a::AgentCard;

pub struct SynapseAgentExecutor {
    agent: Arc<dyn Agent>,
    /// Extension URIs this agent supports (published in its AgentCard).
    ///
    /// Used to negotiate the client's per-request extension request.
    /// Empty means the agent implements no extensions -- which is a real
    /// answer to the client, not a reason to skip the negotiation.
    supported_extensions: Vec<String>,
}

impl SynapseAgentExecutor {
    /// Create a new executor wrapping the given agent.
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        Self {
            agent,
            supported_extensions: Vec::new(),
        }
    }

    /// Declare the extension URIs this agent supports.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.supported_extensions = extensions;
        self
    }

    /// The extension URIs this agent supports.
    pub fn supported_extensions(&self) -> &[String] {
        &self.supported_extensions
    }
}

#[async_trait]
impl AgentExecutor for SynapseAgentExecutor {
    fn execute(
        &self,
        ctx: a2a_server::ExecutorContext,
    ) -> BoxStream<'static, Result<a2a::StreamResponse, a2a::A2AError>> {
        let agent = self.agent.clone();
        let supported_extensions = self.supported_extensions.clone();
        let (tx, rx) = mpsc::channel(32);

        let task_id = ctx.task_id.clone();
        let context_id = ctx.context_id.clone();
        // ⚠⚠ 扩展协商：请求头里的 URI → 与本 agent 支持的交集 ⇒ ⇒
        // 必须在发任何响应之前算好，因为「哪些扩展被采纳」需要
        // 跟着每个事件回显给对端。
        let requested = crate::a2a::extension::requested_extensions(&ctx.service_params);
        let negotiation = crate::a2a::extension::ExtensionNegotiation::negotiate(
            &requested,
            &supported_extensions,
        );
        let message: crate::schema::Message = ctx
            .message
            .map(crate::schema::Message::from_a2a_message)
            .unwrap_or_else(|| crate::schema::Message::user(""));

        tokio::spawn(async move {
            let input = AgentInput {
                messages: vec![message],
                enable_streaming: true,
            };

            let mut stream = agent.stream_run(input).await;
            let mut saw_final = false;
            let mut terminal_sent = false;
            // ⚠⚠ 流式文本的 artifact 是不是已经建过了 ——
            // 第一个 token delta 建，之后的续（见 translate_agent_event_to_a2a）。
            let mut artifact_started = false;

            while let Some(event) = stream.next().await {
                saw_final |= matches!(&event, AgentStreamEvent::Final { .. });
                let response = translate_agent_event_to_a2a(
                    event,
                    &task_id,
                    &context_id,
                    &mut artifact_started,
                );
                terminal_sent |= response_is_terminal(&response);
                let response = crate::a2a::extension::apply_negotiation_metadata(
                    response,
                    &negotiation,
                );
                if tx.send(Ok(response)).await.is_err() {
                    return;
                }
            }

            // ⚠⚠ **终态必须由执行器发出来** ——
            // A2A v1.0 的任务只有在收到一个终态事件（带终态的 `Task`
            // 或 `StatusUpdate`）时才算结束。`ArtifactUpdate` 是**内容**
            // 而不是状态：最终答复是以 artifact 的形式到达的，任务本身
            // 仍然开着。
            //
            // 不发这个事件 ⇒ ⇒ 任务永远停在 `submitted` ⇒ ⇒ 对端
            // （Google ADK 的 `RemoteA2aAgent` 就在其中）按 A2A 规范把
            // 「流在没有到达终态时结束」读成**被截断** ⇒ ⇒ 它抛错，
            // 而不是把答复交回去 ⇒ ⇒ 失败长得像成功。
            if !terminal_sent {
                let response = if saw_final {
                    a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                        task_id: task_id.clone(),
                        context_id: context_id.clone(),
                        status: a2a::TaskStatus {
                            state: a2a::TaskState::Completed,
                            message: None,
                            timestamp: Some(chrono::Utc::now()),
                        },
                        metadata: None,
                    })
                } else {
                    // ⚠⚠ 代理没给出最终答复 ⇒ ⇒ 不得报「完成」。
                    // 报完成就是**失败长得像成功** ⇒ ⇒ 说真话：失败。
                    a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                        task_id: task_id.clone(),
                        context_id: context_id.clone(),
                        status: a2a::TaskStatus {
                            state: a2a::TaskState::Failed,
                            message: Some(
                                crate::schema::Message::assistant(
                                    "the agent's stream ended without a final message",
                                )
                                .into_a2a_message(),
                            ),
                            timestamp: Some(chrono::Utc::now()),
                        },
                        metadata: None,
                    })
                };
                let response = crate::a2a::extension::apply_negotiation_metadata(
                    response,
                    &negotiation,
                );
                let _ = tx.send(Ok(response)).await;
            }
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn cancel(
        &self,
        ctx: a2a_server::ExecutorContext,
    ) -> BoxStream<'static, Result<a2a::StreamResponse, a2a::A2AError>> {
        let task_id = ctx.task_id;
        let context_id = ctx.context_id;

        let stream = async_stream::stream! {
            yield Ok(a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id,
                context_id,
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Canceled,
                    message: None,
                    timestamp: Some(chrono::Utc::now()),
                },
                metadata: None,
            }));
        };

        Box::pin(stream)
    }
}


/// Whether an A2A event ends the interaction.
///
/// A2A v1.0: a task reaches a terminal state only through a `Task` or a
/// `StatusUpdate` that carries a terminal state. An `ArtifactUpdate` is
/// content -- the final message arrives as an artifact, and the task is
/// still open afterwards.
fn response_is_terminal(response: &a2a::StreamResponse) -> bool {
    match response {
        a2a::StreamResponse::Task(task) => task.status.state.is_terminal(),
        a2a::StreamResponse::StatusUpdate(update) => update.status.state.is_terminal(),
        a2a::StreamResponse::ArtifactUpdate(_) | a2a::StreamResponse::Message(_) => false,
    }
}

/// Translate a Synapse [`AgentStreamEvent`] to an A2A [`StreamResponse`].
///
/// `artifact_started` tracks whether the streamed-text artifact already
/// exists on the task: the first token delta **creates** it, the later ones
/// **append** to it.
fn translate_agent_event_to_a2a(
    event: AgentStreamEvent,
    task_id: &str,
    context_id: &str,
    artifact_started: &mut bool,
) -> a2a::StreamResponse {
    use a2a::TaskArtifactUpdateEvent;

    let now = Some(chrono::Utc::now());
    let empty_artifact_id = String::new();

    match event {
        AgentStreamEvent::TokenDelta { agent: _, delta } => {
            // ⚠⚠ **第一个 delta 建 artifact，之后的才 append** ——
            // 服务端 SDK 的规则：`append=true` 要求 artifact **已经存在**
            // （不存在就报 `invalid agent response`，整个任务直接失败），
            // `append=false` 是「建/替换」。
            // ⇒ ⇒ 第一个 delta 走 append=false（建），后面的走
            // append=true（续）—— 顺序反了就是「明明有输出，却整轮失败」。
            let append = *artifact_started;
            *artifact_started = true;
            a2a::StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                artifact: a2a::Artifact {
                    artifact_id: empty_artifact_id.clone(),
                    name: Some("response".to_string()),
                    description: None,
                    parts: vec![a2a::Part::text(delta)],
                    metadata: None,
                    extensions: None,
                },
                append: Some(append),
                last_chunk: Some(false),
                metadata: None,
            })
        }
        AgentStreamEvent::ToolStart {
            agent: _,
            name,
            args,
        } => {
            let text = format!("Calling tool: {}({})", name, args);
            let schema_msg = crate::schema::Message::assistant(text);
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(schema_msg.into_a2a_message()),
                    timestamp: now,
                },
                metadata: tool_event_metadata(&AgentStreamEvent::ToolStart {
                    agent: String::new(),
                    name: name.clone(),
                    args: args.clone(),
                }),
            })
        }
        AgentStreamEvent::ToolEnd {
            agent: _,
            name,
            result,
            latency_ms,
        } => {
            let text = format!("Tool {} returned: {} ({}ms)", name, result, latency_ms);
            let schema_msg = crate::schema::Message::assistant(text);
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(schema_msg.into_a2a_message()),
                    timestamp: now,
                },
                metadata: tool_event_metadata(&AgentStreamEvent::ToolEnd {
                    agent: String::new(),
                    name: name.clone(),
                    result: result.clone(),
                    latency_ms,
                }),
            })
        }
        AgentStreamEvent::ToolCall { agent: _, call } => {
            let text = format!("Calling tool: {}", call.function.name);
            let schema_msg = crate::schema::Message::assistant(text);
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(schema_msg.into_a2a_message()),
                    timestamp: now,
                },
                metadata: tool_event_metadata(&AgentStreamEvent::ToolCall {
                    agent: String::new(),
                    call: call.clone(),
                }),
            })
        }
        AgentStreamEvent::Final { agent: _, message } => {
            let content = match &message.content {
                crate::schema::Content::Text(s) => s.clone(),
                _ => String::new(),
            };
            a2a::StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                artifact: a2a::Artifact {
                    artifact_id: empty_artifact_id.clone(),
                    name: Some("response".to_string()),
                    description: None,
                    parts: vec![a2a::Part::text(content)],
                    metadata: None,
                    extensions: None,
                },
                append: Some(false),
                last_chunk: Some(true),
                metadata: None,
            })
        }
        AgentStreamEvent::Error { agent: _, error } => {
            let text = format!("Agent error: {}", error);
            let schema_msg = crate::schema::Message::assistant(text);
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Failed,
                    message: Some(schema_msg.into_a2a_message()),
                    timestamp: now,
                },
                metadata: None,
            })
        }
        _ => {
            let desc = format!("{:?}", event);
            let schema_msg = crate::schema::Message::assistant(desc);
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(schema_msg.into_a2a_message()),
                    timestamp: now,
                },
                metadata: None,
            })
        }
    }
}

/// Fluent builder for hosting a Synapse agent as an A2A server.
///
/// # Example
///
/// ```rust,ignore
/// # // Requires a concrete Agent impl; see tests for runnable examples.
/// use synapse::a2a::server::A2AServerBuilder;
///
/// // Construct your agent and agent card.
/// // let card = AgentCard::new("my-agent", "A helpful assistant");
/// // let agent: Arc<dyn Agent> = ...;
///
/// // A2AServerBuilder::new(agent, card)
/// //     .bind("127.0.0.1:8080".parse()?)
/// //     .serve()
/// //     .await?;
/// ```
/// ⚠️⚠️ **默认监听地址 —— 必须是回环**。
///
/// ⇒ ⇒ **宽绑定（`0.0.0.0`）必须显式给**（`.bind(...)`）⇒ ⇒ 理由有二：
/// ⇒ ⇒ **① 暴露面**（默认安装不该在局域网上可读）；
/// ⇒ ⇒ **② 形状** —— 宽绑定时**报出去的**地址若也是 `0.0.0.0`
/// ⇒ ⇒ ⇒ ⇒ 那不是「别人能用来连我」的地址 ⇒ ⇒ 单机上碰巧能通、
/// ⇒ ⇒ ⇒ ⇒ 换台机器**永远连不上**。
pub const DEFAULT_A2A_BIND: &str = "127.0.0.1:8080";

pub struct A2AServerBuilder {
    agent: Arc<dyn Agent>,
    card: AgentCard,
    bind_addr: Option<std::net::SocketAddr>,
}

impl A2AServerBuilder {
    pub fn new(agent: Arc<dyn Agent>, card: AgentCard) -> Self {
        Self {
            agent,
            card,
            bind_addr: None,
        }
    }

    pub fn bind(mut self, addr: std::net::SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.serve_with_store(InMemoryTaskStore::new()).await
    }

    /// Bind the given address, start serving in the background, and report
    /// the address actually bound.
    ///
    /// ⚠⚠ **报出“实际绑的地址”而不是“要绑的地址”** ⇒ ⇒
    /// 传 0.0.0.0:0 时实际绑到的是某个具体端口（不是可连接地址），
    /// 而调用方想要的是「零端口已占用 ⇒ 拿到哪个端口」。
    /// 两者不是一件事 ⇒ ⇒ 返回实际绑定地址才能同时满足两边。
    ///
    /// 服务在后台任务里跑（`tokio::spawn`），所以本函数**立即返回** ⇒ ⇒
    /// 调用方拿到的地址立刻可用；这与 [`Self::serve`] 的阻塞语义不同，
    /// 别把两者混用：
    ///
    /// * [`Self::serve`] —— 阻塞直到服务结束（生产用）。
    /// * [`Self::serve_on`] —— 后台服务 + 立刻返回实际地址（测试与
    ///   「先绑后公布」用）。
    pub async fn serve_on(
        self,
        addr: std::net::SocketAddr,
    ) -> Result<std::net::SocketAddr, Box<dyn std::error::Error + Send + Sync>> {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("bind {addr}: {e}"))?;
        let bound = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {e}"))?;
        let app = self.build_app();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(bound)
    }

    /// Build the axum router (JSON-RPC + REST + agent card).
    ///
    /// ⚠⚠ **扩展接线只有这一处** ⇒ ⇒ 卡片声明的扩展集与
    /// 执行器实际支持的扩展集同源 ⇒ ⇒ 两边不会走偏。
    /// 写成两份就会出现「卡片里写着支持 X，但执行器不会」
    /// ⇒ ⇒ 协商失配。
    fn build_app_with_store<S: a2a_server::TaskStore>(
        &self,
        task_store: S,
    ) -> axum::Router {
        // ⚠⚠ **卡片必须说实话** —— 这个 server 只有一条执行路径
        // （代理的 `stream_run`），而且 JSON-RPC 路由确实提供
        // `SendStreamingMessage` ⇒ ⇒ 「不支持流式」是假话。
        //
        // 假话的代价是实打实的：客户端（a2a-sdk、ADK）读到
        // `streaming: false` 就退回单发路径，而 A2A v1.0 的单发路径
        // 返回的是**从 task_store 重新加载**的 Task ⇒ ⇒ 事件上的
        // metadata（扩展协商的结果）在那条路上整个丢掉 ⇒ ⇒ 客户端
        // 分不清「对端不支持」与「对端忽略了」。
        let mut card = self.card.clone();
        card.capabilities.streaming = true;

        let executor = SynapseAgentExecutor::new(self.agent.clone())
            .with_extensions(card.capabilities.extensions.clone());
        // ⚠⚠ 能力闸与卡片**同源** —— 卡片说能流式，服务端就得真能；
        // 两处各写一份就会出现「卡片说一套、服务端做另一套」。
        // `push_notifications` 保持未声明（None = 不限制，SDK 默认），
        // 因为这个 server 根本没实现推送通知。
        let handler = DefaultRequestHandler::new(executor, task_store)
            .with_capabilities(a2a::AgentCapabilities {
                streaming: Some(card.capabilities.streaming),
                ..Default::default()
            });

        let handler = Arc::new(handler);
        let mut app = a2a_server::jsonrpc::jsonrpc_router(handler.clone());
        app = app.merge(a2a_server::rest::rest_router(handler));
        app = app.merge(a2a_server::agent_card::agent_card_router(Arc::new(
            a2a_server::StaticAgentCard::new(card.into_a2a_card()),
        )));
        app
    }

    /// Build the axum router with the default in-memory task store.
    fn build_app(&self) -> axum::Router {
        self.build_app_with_store(InMemoryTaskStore::new())
    }

    pub async fn serve_with_store<S: a2a_server::TaskStore>(
        self,
        task_store: S,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bind_addr = self
            .bind_addr
            .unwrap_or_else(|| DEFAULT_A2A_BIND.parse().unwrap());

        let app = self.build_app_with_store(task_store);
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Content, Message, RoleType};

    /// ⚠️⚠️ **默认监听必须是回环** —— 宽绑定必须显式（`.bind(...)`）。
    ///
    /// ⇒ ⇒ 这条钉的是**契约** ⇒ ⇒ 谁把默认值悄悄改回 `0.0.0.0`，
    /// ⇒ ⇒ 这条当场红 ⇒ ⇒ 而不是等某台机器上「局域网可读」。
    #[test]
    fn the_default_bind_is_loopback() {
        let addr: std::net::SocketAddr =
            DEFAULT_A2A_BIND.parse().expect("默认地址必须可解析");
        assert!(
            addr.ip().is_loopback(),
            "默认监听必须回环（{DEFAULT_A2A_BIND}）⇒ 宽绑定必须显式"
        );
    }

    /// Translate one event the way the executor does, with a fresh
    /// artifact state (no artifact has been created yet).
    fn translate(event: AgentStreamEvent) -> a2a::StreamResponse {
        let mut artifact_started = false;
        translate_agent_event_to_a2a(event, "task-1", "ctx-1", &mut artifact_started)
    }
    #[test]
    fn the_first_token_delta_creates_the_artifact_and_later_ones_append() {
        let mut started = false;

        let first = translate_agent_event_to_a2a(
            AgentStreamEvent::TokenDelta {
                agent: "x".into(),
                delta: "hello".into(),
            },
            "task-1",
            "ctx-1",
            &mut started,
        );
        match first {
            a2a::StreamResponse::ArtifactUpdate(e) => {
                assert_eq!(e.task_id, "task-1");
                assert_eq!(e.context_id, "ctx-1");
                assert_eq!(
                    e.append,
                    Some(false),
                    "the first delta must create the artifact, not append to it"
                );
                assert_eq!(e.last_chunk, Some(false));
                assert_eq!(e.artifact.parts.len(), 1);
            }
            _ => panic!("expected ArtifactUpdate"),
        }

        let second = translate_agent_event_to_a2a(
            AgentStreamEvent::TokenDelta {
                agent: "x".into(),
                delta: " world".into(),
            },
            "task-1",
            "ctx-1",
            &mut started,
        );
        match second {
            a2a::StreamResponse::ArtifactUpdate(e) => {
                assert_eq!(
                    e.append,
                    Some(true),
                    "later deltas must append to the artifact that already exists"
                );
            }
            _ => panic!("expected ArtifactUpdate"),
        }
    }

    #[test]
    fn final_event_becomes_last_chunk_artifact() {
        let event = AgentStreamEvent::Final {
            agent: "x".into(),
            message: Message {
                role: RoleType::Assistant,
                content: Content::Text("done".into()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
        };
        let response = translate(event);
        match response {
            a2a::StreamResponse::ArtifactUpdate(e) => {
                assert_eq!(e.append, Some(false));
                assert_eq!(e.last_chunk, Some(true));
            }
            _ => panic!("expected ArtifactUpdate"),
        }
    }

    #[test]
    fn error_event_becomes_failed_status() {
        let event = AgentStreamEvent::Error {
            agent: "x".into(),
            error: crate::adk::agent::AgentError::Agent("oops".into()),
        };
        let response = translate(event);
        match response {
            a2a::StreamResponse::StatusUpdate(e) => {
                assert_eq!(e.status.state, a2a::TaskState::Failed);
                let msg = e.status.message.expect("error message present");
                match msg.parts.first().map(|p| &p.content) {
                    Some(a2a::PartContent::Text(t)) => assert!(t.contains("oops")),
                    _ => panic!("expected Text part"),
                }
            }
            _ => panic!("expected StatusUpdate"),
        }
    }

    #[test]
    fn tool_start_becomes_working_status() {
        let event = AgentStreamEvent::ToolStart {
            agent: "x".into(),
            name: "search".into(),
            args: "{}".into(),
        };
        let response = translate(event);
        match response {
            a2a::StreamResponse::StatusUpdate(e) => {
                assert_eq!(e.status.state, a2a::TaskState::Working);
            }
            _ => panic!("expected StatusUpdate"),
        }
    }

    #[test]
    fn tool_start_carries_structured_tool_event_in_metadata() {
        let event = AgentStreamEvent::ToolStart {
            agent: "x".into(),
            name: "search".into(),
            args: "{\"q\":\"rust\"}".into(),
        };
        let response = translate(event);
        match response {
            a2a::StreamResponse::StatusUpdate(e) => {
                let metadata = e.metadata.expect("tool event must ride in metadata");
                let decoded = crate::a2a::tool_event::tool_event_from_metadata(Some(&metadata))
                    .expect("tool event must decode");
                match decoded {
                    crate::a2a::tool_event::ToolEvent::Start { name, args } => {
                        assert_eq!(name, "search");
                        assert_eq!(args, "{\"q\":\"rust\"}");
                    }
                    other => panic!("expected Start, got {other:?}"),
                }
            }
            _ => panic!("expected StatusUpdate"),
        }
    }


    /// ⚠⚠ 卡片不得声称不支持协商所需的协议版本。
    /// 旧实现把 protocolVersion 写死为 "0.3" ⇒ ⇒
    /// 对端按 A2A v1.0 规范去协商就会落到旧版本上。
    #[test]
    fn the_card_advertises_a2a_v1() {
        let card = AgentCard::new("a", "d", "http://x", "1.0.0");
        let sdk = card.into_a2a_card();
        assert_eq!(sdk.supported_interfaces.len(), 1);
        assert_eq!(sdk.supported_interfaces[0].protocol_version, "1.0");
    }

    /// 卡片里声明的扩展必须真的到达对端。
    #[test]
    fn the_card_publishes_declared_extensions() {
        let mut card = AgentCard::new("a", "d", "http://x", "1.0.0");
        card.capabilities.extensions = vec!["https://example.com/ext/v1".to_string()];
        let sdk = card.into_a2a_card();
        let exts = sdk.capabilities.extensions.expect("extensions must reach the wire");
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0].uri, "https://example.com/ext/v1");
    }

    /// 扩展必须能原道返回来 ⇒ ⇒ 否则一个对端声称支持 X、
    /// 实际不支持的情况就会被宣称出去。
    #[test]
    fn extensions_round_trip_through_the_card() {
        let mut card = AgentCard::new("a", "d", "http://x", "1.0.0");
        card.capabilities.extensions = vec!["https://example.com/ext/v1".to_string()];
        let back = AgentCard::from_a2a_card(card.into_a2a_card());
        assert_eq!(back.capabilities.extensions, vec!["https://example.com/ext/v1"]);
    }

    /// ⚠⚠ 端到端：请求头 → 协商 → 回显。
    /// 这条判据钉的是「头到了服务端」而不是「头被忽略了」。
    #[test]
    fn the_request_header_reaches_the_negotiation_and_is_echoed_back() {
        let mut headers: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        headers.insert(
            crate::a2a::extension::HEADER_X_A2A_EXTENSIONS.to_string(),
            vec!["https://example.com/ext/v1".to_string()],
        );
        let requested = crate::a2a::extension::requested_extensions(&headers);
        let negotiation = crate::a2a::extension::ExtensionNegotiation::negotiate(
            &requested,
            &["https://example.com/ext/v1".to_string()],
        );
        assert_eq!(negotiation.activated, vec!["https://example.com/ext/v1"]);

        // 回显到线上的事件里
        let response = crate::a2a::extension::apply_negotiation_metadata(
            a2a::StreamResponse::ArtifactUpdate(a2a::TaskArtifactUpdateEvent {
                task_id: "t1".into(),
                context_id: "c1".into(),
                artifact: a2a::Artifact {
                    artifact_id: "".into(),
                    name: None,
                    description: None,
                    parts: vec![a2a::Part::text("hi")],
                    metadata: None,
                    extensions: None,
                },
                append: Some(true),
                last_chunk: Some(false),
                metadata: None,
            }),
            &negotiation,
        );
        match response {
            a2a::StreamResponse::ArtifactUpdate(e) => {
                let meta = e.metadata.expect("negotiation must ride in metadata");
                let activated = meta
                    .get(crate::a2a::extension::METADATA_KEY_ACTIVATED_EXTENSIONS)
                    .and_then(|v| v.as_array())
                    .expect("activatedExtensions must be present");
                assert_eq!(activated.len(), 1);
                assert_eq!(activated[0], "https://example.com/ext/v1");
            }
            _ => panic!("expected ArtifactUpdate"),
        }
    }

    /// 不支持的扩展必须报出来 ⇒ ⇒ 否则客户端会误以为
    /// 自己的请求被采纳了。
    #[test]
    fn unsupported_extensions_are_reported_not_swallowed() {
        let requested = vec!["https://example.com/ext/v1".to_string()];
        let negotiation =
            crate::a2a::extension::ExtensionNegotiation::negotiate(&requested, &[]);
        assert!(negotiation.activated.is_empty());
        assert_eq!(negotiation.unsupported, vec!["https://example.com/ext/v1"]);

        let response = crate::a2a::extension::apply_negotiation_metadata(
            a2a::StreamResponse::StatusUpdate(a2a::TaskStatusUpdateEvent {
                task_id: "t1".into(),
                context_id: "c1".into(),
                status: a2a::TaskStatus {
                    state: a2a::TaskState::Working,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }),
            &negotiation,
        );
        match response {
            a2a::StreamResponse::StatusUpdate(e) => {
                let meta = e.metadata.expect("metadata must be present");
                assert!(!meta.contains_key("activatedExtensions"));
                let unsupported = meta
                    .get("unsupportedExtensions")
                    .and_then(|v| v.as_array())
                    .expect("unsupportedExtensions must be reported");
                assert_eq!(unsupported.len(), 1);
            }
            _ => panic!("expected StatusUpdate"),
        }
    }

    /// 工具事件的旁路元数据不能被扩展协商覆盖丢掉 ⇒ ⇒
    /// 两边都是「消息」，覆盖了就算丢了工具调用。
    #[test]
    fn extension_metadata_does_not_clobber_the_tool_event_channel() {
        let event = AgentStreamEvent::ToolStart {
            agent: "x".into(),
            name: "search".into(),
            args: "{}".into(),
        };
        let response = translate(event);
        let negotiation = crate::a2a::extension::ExtensionNegotiation {
            activated: vec!["https://example.com/ext/v1".to_string()],
            unsupported: vec![],
        };
        let response = crate::a2a::extension::apply_negotiation_metadata(response, &negotiation);
        match response {
            a2a::StreamResponse::StatusUpdate(e) => {
                let meta = e.metadata.expect("metadata must be present");
                assert!(
                    crate::a2a::tool_event::tool_event_from_metadata(Some(&meta)).is_some(),
                    "the tool event must survive the extension metadata merge"
                );
                assert!(meta.contains_key(crate::a2a::extension::METADATA_KEY_ACTIVATED_EXTENSIONS));
            }
            _ => panic!("expected StatusUpdate"),
        }
    }
    /// The regression this whole channel exists for: a tool call must
    /// survive a full A2A hop — server event → wire JSON → client
    /// event. Before the structured channel the server flattened the
    /// tool call to text and the client dropped the status update, so
    /// the downstream consumer never saw a tool event at all.
    #[test]
    fn tool_call_survives_the_full_a2a_hop() {
        let event = AgentStreamEvent::ToolStart {
            agent: "x".into(),
            name: "set_theme".into(),
            args: "{\"color\":\"#71717A\"}".into(),
        };

        // server → wire
        let response = translate(event);
        let wire = serde_json::to_value(&response).expect("serialize wire response");

        // wire → client
        let decoded: a2a::StreamResponse =
            serde_json::from_value(wire).expect("deserialize wire response");
        let events = crate::a2a::client::translate_a2a_response_to_agent_events(
            decoded,
            "http://peer:9000",
            &mut crate::a2a::client::StreamProgress::default(),
        );

        assert_eq!(events.len(), 1, "exactly one event must come back");
        match &events[0] {
            AgentStreamEvent::ToolStart { agent, name, args } => {
                assert_eq!(agent, "http://peer:9000");
                assert_eq!(name, "set_theme");
                assert_eq!(args, "{\"color\":\"#71717A\"}");
            }
            other => panic!("expected ToolStart after the hop, got {other:?}"),
        }
    }
}
