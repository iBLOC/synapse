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
use crate::schema::Message;

/// Adapts a Synapse [`Agent`] to the a2a-rs [`AgentExecutor`] trait.
///
/// The executor is the bridge between the A2A protocol layer (a2a-rs SDK)
/// and the agent's internal execution model. When an A2A client sends a
/// `message/send` or `message/stream` request, the SDK invokes
/// [`execute`](AgentExecutor::execute) on this executor, which in turn
/// calls the agent's `stream_run` method.
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
    async fn execute(
        &self,
        task_id: String,
        context_id: String,
        message: Message,
        _metadata: Option<serde_json::Value>,
    ) -> BoxStream<'static, Result<a2a::StreamResponse, a2a::Error>> {
        let agent = self.agent.clone();
        let (tx, rx) = mpsc::channel(32);

        // Spawn the agent's streaming execution
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

    async fn cancel(
        &self,
        task_id: String,
        context_id: String,
    ) -> Result<a2a::Task, a2a::Error> {
        // For now, return a canceled task. Future: propagate cancellation to agent.
        Ok(a2a::Task {
            id: task_id,
            context_id,
            status: a2a::TaskStatus {
                state: a2a::TaskState::Canceled,
                message: None,
                timestamp: chrono::Utc::now(),
            },
            history: None,
            artifacts: None,
            metadata: None,
        })
    }
}

/// Translate a Synapse [`AgentStreamEvent`] to an A2A [`StreamResponse`].
fn translate_agent_event_to_a2a(
    event: AgentStreamEvent,
    task_id: &str,
    context_id: &str,
) -> a2a::StreamResponse {
    use a2a::{ArtifactUpdateEvent, Part, StatusUpdateEvent, TaskStatus};

    match event {
        AgentStreamEvent::TokenDelta { agent: _, delta } => {
            // Emit as an artifact update with append=true
            a2a::StreamResponse::ArtifactUpdate(ArtifactUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                artifact: a2a::Artifact {
                    name: Some("response".to_string()),
                    parts: vec![Part::Text(delta)],
                    metadata: None,
                },
                append: true,
                last_chunk: false,
                metadata: None,
            })
        }
        AgentStreamEvent::ToolStart { agent: _, name, args } => {
            // Emit as a status update with working state
            let msg = format!("Calling tool: {}({})", name, args);
            a2a::StreamResponse::StatusUpdate(StatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(Message::assistant(msg)),
                    timestamp: chrono::Utc::now(),
                },
                metadata: None,
            })
        }
        AgentStreamEvent::ToolEnd { agent: _, name, result, latency_ms } => {
            let msg = format!("Tool {} returned: {} ({}ms)", name, result, latency_ms);
            a2a::StreamResponse::StatusUpdate(StatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(Message::assistant(msg)),
                    timestamp: chrono::Utc::now(),
                },
                metadata: None,
            })
        }
        AgentStreamEvent::Final { agent: _, message } => {
            // Emit final artifact + completed status
            let content = match &message.content {
                crate::schema::Content::Text(s) => s.clone(),
                _ => String::new(),
            };
            a2a::StreamResponse::ArtifactUpdate(ArtifactUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                artifact: a2a::Artifact {
                    name: Some("response".to_string()),
                    parts: vec![Part::Text(content)],
                    metadata: None,
                },
                append: false,
                last_chunk: true,
                metadata: None,
            })
        }
        AgentStreamEvent::Error { agent: _, error } => {
            let msg = format!("Agent error: {}", error);
            a2a::StreamResponse::StatusUpdate(StatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: TaskStatus {
                    state: a2a::TaskState::Failed,
                    message: Some(Message::assistant(msg)),
                    timestamp: chrono::Utc::now(),
                },
                metadata: None,
            })
        }
        // Other events (StepTransition, RouteDecision, Handoff) are internal
        // observability; not exposed to A2A wire protocol.
        _ => {
            // Emit as a working status update with a description
            let desc = format!("{:?}", event);
            a2a::StreamResponse::StatusUpdate(StatusUpdateEvent {
                task_id: task_id.to_string(),
                context_id: context_id.to_string(),
                status: TaskStatus {
                    state: a2a::TaskState::Working,
                    message: Some(Message::assistant(desc)),
                    timestamp: chrono::Utc::now(),
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
/// ```rust,no_run
/// use synapse::a2a::server::A2AServerBuilder;
/// use synapse::a2a::AgentCard;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let agent = Arc::new(my_agent);
/// let card = AgentCard::new("my-agent", "A helpful assistant");
///
/// A2AServerBuilder::new(agent, card)
///     .bind("0.0.0.0:8080".parse()?)
///     .serve()
///     .await?;
/// # Ok(())
/// # }
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

        let router = a2a_server::router::build(handler, self.card.into_a2a_card());

        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}
