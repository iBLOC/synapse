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
use crate::a2a::AgentCard;

pub struct SynapseAgentExecutor {
    agent: Arc<dyn Agent>,
}

impl SynapseAgentExecutor {
    /// Create a new executor wrapping the given agent.
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl AgentExecutor for SynapseAgentExecutor {
    fn execute(
        &self,
        ctx: a2a_server::ExecutorContext,
    ) -> BoxStream<'static, Result<a2a::StreamResponse, a2a::A2AError>> {
        let agent = self.agent.clone();
        let (tx, rx) = mpsc::channel(32);

        let task_id = ctx.task_id.clone();
        let context_id = ctx.context_id.clone();
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

            while let Some(event) = stream.next().await {
                let response = translate_agent_event_to_a2a(event, &task_id, &context_id);
                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
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

/// Translate a Synapse [`AgentStreamEvent`] to an A2A [`StreamResponse`].
fn translate_agent_event_to_a2a(
    event: AgentStreamEvent,
    task_id: &str,
    context_id: &str,
) -> a2a::StreamResponse {
    use a2a::TaskArtifactUpdateEvent;

    let now = Some(chrono::Utc::now());
    let empty_artifact_id = String::new();

    match event {
        AgentStreamEvent::TokenDelta { agent: _, delta } => {
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
                append: Some(true),
                last_chunk: Some(false),
                metadata: None,
            })
        }
        AgentStreamEvent::ToolStart { agent: _, name, args } => {
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
                metadata: None,
            })
        }
        AgentStreamEvent::ToolEnd { agent: _, name, result, latency_ms } => {
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
                metadata: None,
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
/// //     .bind("0.0.0.0:8080".parse()?)
/// //     .serve()
/// //     .await?;
/// ```
pub struct A2AServerBuilder {
    agent: Arc<dyn Agent>,
    card: AgentCard,
    bind_addr: Option<std::net::SocketAddr>,
}

impl A2AServerBuilder {
    /// Create a new server builder for the given agent and card.
    pub fn new(agent: Arc<dyn Agent>, card: AgentCard) -> Self {
        Self {
            agent,
            card,
            bind_addr: None,
        }
    }

    /// Set the bind address for the server.
    pub fn bind(mut self, addr: std::net::SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    /// Build and serve the A2A server.
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bind_addr = self.bind_addr.unwrap_or_else(|| "0.0.0.0:8080".parse().unwrap());

        let executor = SynapseAgentExecutor::new(self.agent);
        let task_store = InMemoryTaskStore::new();
        let handler = DefaultRequestHandler::new(executor, task_store);

        let handler = Arc::new(handler);
        let mut app = a2a_server::jsonrpc::jsonrpc_router(handler.clone());
        app = app.merge(a2a_server::rest::rest_router(handler));
        app = app.merge(a2a_server::agent_card::agent_card_router(
            Arc::new(a2a_server::StaticAgentCard::new(self.card.into_a2a_card()))
        ));

        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
