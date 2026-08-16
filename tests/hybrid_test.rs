use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use synapse::adk::agent::{
    Agent, AgentAction, AgentError, AgentEvent, AgentInput, AgentOutput, AgentResult,
    ResumeInfo,
};
use synapse::adk::multiagent::HybridSupervisor;
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

struct MockSubAgent {
    name: String,
    description: String,
    response: String,
    transfer_to: Option<String>,
    received_tasks: Arc<Mutex<Vec<String>>>,
}

impl MockSubAgent {
    fn new(
        name: &str,
        description: &str,
        response: &str,
        transfer_to: Option<String>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name: name.to_string(),
            description: description.to_string(),
            response: response.to_string(),
            transfer_to,
            received_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl Agent for MockSubAgent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    async fn run(&self, input: AgentInput) -> AgentResult<AgentEvent> {
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
                action: Some(AgentAction {
                    exit: false,
                    transfer_to: self.transfer_to.clone(),
                    interrupted: None,
                }),
            },
            error: None,
        })
    }
    async fn resume(
        &self,
        _checkpoint: Vec<u8>,
        _info: ResumeInfo,
    ) -> AgentResult<AgentEvent> {
        unimplemented!()
    }
}

fn tool_call(name: &str, args: &str) -> ToolCall {
    ToolCall {
        id: format!("call_{}", name),
        tool_type: "function".to_string(),
        function: FunctionCall {
            name: name.to_string(),
            arguments: args.to_string(),
        },
    }
}

fn message_with_tool_calls(calls: Vec<ToolCall>) -> Message {
    Message {
        role: RoleType::Assistant,
        content: Content::Text("".to_string()),
        name: None,
        tool_call_id: None,
        tool_calls: Some(calls),
    }
}

#[tokio::test]
async fn test_hybrid_router_only() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_a", r#"{"task": "do X"}"#)]),
        Message::assistant("A answered, done"),
    ]);

    let a = MockSubAgent::new("a", "Agent A", "A's answer", None);
    let supervisor = HybridSupervisor::builder("hybrid", model)
        .add_agent("a", a.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Use a")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");
    let msg = result.output.message.expect("message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("A answered"), "got: {}", s);
    } else {
        panic!("expected text");
    }
    assert_eq!(a.received_tasks.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_hybrid_with_handoff() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_billing", r#"{"task": "billing issue"}"#)]),
        Message::assistant("Final answer based on refund agent"),
    ]);

    let billing = MockSubAgent::new(
        "billing",
        "Billing questions",
        "Billing: this is a refund",
        Some("refund".to_string()),
    );
    let refund = MockSubAgent::new(
        "refund",
        "Refund questions",
        "Refund: processed",
        None,
    );

    let supervisor = HybridSupervisor::builder("hybrid", model)
        .add_agent("billing", billing.clone() as Arc<dyn Agent>)
        .add_agent("refund", refund.clone() as Arc<dyn Agent>)
        .max_handoffs(3)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("I want money back")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");
    let msg = result.output.message.expect("message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Final answer"), "got: {}", s);
    } else {
        panic!("expected text");
    }
    assert_eq!(billing.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(refund.received_tasks.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_hybrid_multi_handoff_chain() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_a", r#"{"task": "start"}"#)]),
        Message::assistant("Final from c"),
    ]);

    let a = MockSubAgent::new("a", "A", "A out", Some("b".to_string()));
    let b = MockSubAgent::new("b", "B", "B out", Some("c".to_string()));
    let c = MockSubAgent::new("c", "C", "C final", None);

    let supervisor = HybridSupervisor::builder("hybrid", model)
        .add_agent("a", a.clone() as Arc<dyn Agent>)
        .add_agent("b", b.clone() as Arc<dyn Agent>)
        .add_agent("c", c.clone() as Arc<dyn Agent>)
        .max_handoffs(5)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Chain handoff")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await.expect("run failed");
    let msg = result.output.message.expect("message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Final from c"), "got: {}", s);
    } else {
        panic!("expected text");
    }
    assert_eq!(a.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(b.received_tasks.lock().unwrap().len(), 1);
    assert_eq!(c.received_tasks.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_hybrid_max_handoff_exceeded() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_a", r#"{"task": "loop"}"#)]),
    ]);

    let a = MockSubAgent::new("a", "A", "A", Some("a".to_string()));

    let supervisor = HybridSupervisor::builder("hybrid", model)
        .add_agent("a", a.clone() as Arc<dyn Agent>)
        .max_handoffs(2)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Loop")],
        enable_streaming: false,
    };
    let result = supervisor.run(input).await;
    assert!(matches!(result, Err(AgentError::Agent(msg)) if msg.contains("max handoffs")));
}
