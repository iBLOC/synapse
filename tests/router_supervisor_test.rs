use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo};
use synapse::adk::multiagent::RouterSupervisor;
use synapse::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use synapse::components::model::StreamReader;
use synapse::schema::{Content, FunctionCall, Message, RoleType, ToolCall};

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
        if self.responses.is_empty() {
            return Err(ModelError::Api("No responses configured".to_string()));
        }
        let i = idx % self.responses.len();
        Ok(self.responses[i].clone())
    }

    async fn stream(
        &self,
        _messages: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<StreamReader> {
        Err(ModelError::Api("Streaming not implemented".to_string()))
    }
}

struct MockAgent {
    name: String,
    description: String,
    response: String,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

impl MockAgent {
    fn new(name: &str, description: &str, response: &str) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            description: description.to_string(),
            response: response.to_string(),
            received_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl Agent for MockAgent {
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
            .and_then(|m| match &m.content {
                Content::Text(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.received_tasks.lock().unwrap().push(task);
        Ok(AgentEvent {
            agent_name: self.name.clone(),
            output: AgentOutput {
                message: Some(Message::assistant(self.response.clone())),
                action: None,
            },
            error: None,
        })
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

struct FailingMockAgent {
    name: String,
    description: String,
}

#[async_trait]
impl Agent for FailingMockAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, _input: AgentInput) -> Result<AgentEvent, AgentError> {
        Err(AgentError::Agent("mock failure".to_string()))
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> Result<AgentEvent, AgentError> {
        unimplemented!()
    }
}

fn tool_call_message(name: &str, args: &str) -> Message {
    Message {
        role: RoleType::Assistant,
        content: Content::Text("".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(vec![ToolCall {
            id: format!("call_{}", name),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: name.to_string(),
                arguments: args.to_string(),
            },
        }]),
    }
}

#[tokio::test]
async fn test_router_routes_to_single_agent() {
    let model = MockChatModel::new(vec![
        tool_call_message("route_to_researcher", r#"{"task": "capital of France?"}"#),
        Message::assistant("The answer is Paris."),
    ]);

    let agent = MockAgent::new("researcher", "Researches topics", "Paris is the capital");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(agent.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("What is the capital of France?")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    let received = agent.received_tasks.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "capital of France?");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Paris"), "got: {}", s);
    } else {
        panic!("expected text content");
    }
}

#[tokio::test]
async fn test_router_routes_to_multiple_agents() {
    let model = MockChatModel::new(vec![
        tool_call_message("route_to_researcher", r#"{"task": "Q1"}"#),
        tool_call_message("route_to_calc", r#"{"task": "Q2"}"#),
        Message::assistant("Both done."),
    ]);

    let researcher = MockAgent::new("researcher", "Researches", "R answer");
    let calc = MockAgent::new("calc", "Calculates", "C answer");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(researcher.clone() as Arc<dyn Agent>)
        .add_agent(calc.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Q1 and Q2")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    assert_eq!(researcher.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(calc.received_tasks.lock().unwrap().len(), 1);
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Both done"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_router_no_route_needed() {
    let model = MockChatModel::new(vec![Message::assistant("Direct answer, no agent needed")]);
    let agent = MockAgent::new("a", "A", "A");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(agent.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Hi")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    assert_eq!(agent.received_tasks.lock().unwrap().len(), 0);
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert_eq!(s, "Direct answer, no agent needed");
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_router_handles_agent_failure() {
    let model = MockChatModel::new(vec![
        tool_call_message("route_to_broken", r#"{"task": "X"}"#),
        Message::assistant("Could not complete"),
    ]);

    let failing: Arc<dyn Agent> = Arc::new(FailingMockAgent {
        name: "broken".to_string(),
        description: "Always fails".to_string(),
    });
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .add_agent(failing)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Try X")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Could not complete"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_router_max_iterations_exceeded() {
    let model = MockChatModel::new(vec![tool_call_message(
        "route_to_a",
        r#"{"task": "loop"}"#,
    )]);

    let agent = MockAgent::new("a", "A", "A");
    let supervisor = RouterSupervisor::builder("supervisor", model)
        .max_iterations(3)
        .add_agent(agent as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Loop")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await;
    assert!(matches!(result, Err(AgentError::MaxIterationsExceeded)));
}

#[tokio::test]
async fn test_router_nested() {
    let inner_model = MockChatModel::new(vec![
        tool_call_message("route_to_sub", r#"{"task": "inner task"}"#),
        Message::assistant("Inner result"),
    ]);

    let outer_model = MockChatModel::new(vec![
        tool_call_message("route_to_inner", r#"{"task": "outer task"}"#),
        Message::assistant("Outer synthesized"),
    ]);

    let sub = MockAgent::new("sub", "Sub agent", "Sub result");
    let inner_supervisor = RouterSupervisor::builder("inner", inner_model)
        .add_agent(sub.clone() as Arc<dyn Agent>)
        .build();

    let outer_supervisor = RouterSupervisor::builder("outer", outer_model)
        .add_agent(Arc::new(inner_supervisor) as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Nested test")],
        enable_streaming: false,
    };
    let result = outer_supervisor.run(input).await.expect("run failed");

    let received = sub.received_tasks.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "inner task");
    drop(received);

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Outer synthesized"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_router_empty_agents() {
    let model = MockChatModel::new(vec![Message::assistant("Pure LLM answer")]);
    let supervisor = RouterSupervisor::builder("supervisor", model).build();

    let input = AgentInput {
        messages: vec![Message::user("Hi")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert_eq!(s, "Pure LLM answer");
    } else {
        panic!("expected text");
    }
}
