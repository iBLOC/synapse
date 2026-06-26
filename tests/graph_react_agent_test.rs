use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use synapse::adk::agent::react::GraphReActAgent;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use synapse::schema::{Content, Message, RoleType};

struct MockChatModel {
    responses: Vec<Message>,
    index: AtomicUsize,
}

impl MockChatModel {
    fn new(responses: Vec<Message>) -> Arc<Self> {
        Arc::new(Self {
            responses,
            index: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl BaseChatModel for MockChatModel {
    async fn generate(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<Message> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            Ok(self.responses[idx].clone())
        } else {
            Err(ModelError::Api("No more responses".to_string()))
        }
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<synapse::components::model::StreamReader> {
        Err(ModelError::Api("Streaming not implemented".to_string()))
    }
}

#[tokio::test]
async fn test_graph_react_agent_builder() {
    let model = MockChatModel::new(vec![Message::assistant("Hello! How can I help you?")]);

    let agent = GraphReActAgent::builder()
        .name("TestAgent")
        .description("A test agent")
        .with_model(model)
        .max_steps(5)
        .build();

    assert!(agent.is_ok());
}

#[tokio::test]
async fn test_graph_react_agent_name_and_description() {
    let model = MockChatModel::new(vec![Message::assistant("Hi")]);

    let agent = GraphReActAgent::builder()
        .name("MyAgent")
        .description("My test agent description")
        .with_model(model)
        .build()
        .unwrap();

    assert_eq!(agent.name(), "MyAgent");
    assert_eq!(agent.description(), "My test agent description");
}
