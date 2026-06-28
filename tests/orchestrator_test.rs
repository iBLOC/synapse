use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use synapse::adk::agent::{Agent, AgentError, AgentEvent, AgentInput, AgentOutput, ResumeInfo};
use synapse::adk::multiagent::OrchestratorAgent;
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
async fn test_orchestrator_decomposes_and_dispatches() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_researcher", r#"{"task": "find X"}"#)]),
        Message::assistant("Synthesized final answer"),
    ]);

    let agent = MockAgent::new("researcher", "Researches topics", "X is found");
    let orchestrator = OrchestratorAgent::builder("orchestrator", model)
        .add_agent(agent.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Build a report on X")],
        enable_streaming: false,
    };
    let result = orchestrator.run(input).await.expect("run failed");

    assert_eq!(agent.received_tasks.lock().unwrap().len(), 1);
    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Synthesized"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_orchestrator_parallel_dispatch() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![
            tool_call("route_to_researcher", r#"{"task": "research Q1"}"#),
            tool_call("route_to_analyst", r#"{"task": "analyze Q2"}"#),
        ]),
        Message::assistant("Synthesized from both"),
    ]);

    let researcher = MockAgent::new("researcher", "Researches", "R result");
    let analyst = MockAgent::new("analyst", "Analyzes", "A result");
    let orchestrator = OrchestratorAgent::builder("orchestrator", model)
        .add_agent(researcher.clone() as Arc<dyn Agent>)
        .add_agent(analyst.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Do both tasks in parallel")],
        enable_streaming: false,
    };
    let result = orchestrator.run(input).await.expect("run failed");

    let r = researcher.received_tasks.lock().unwrap();
    let a = analyst.received_tasks.lock().unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(a.len(), 1);
    assert_eq!(r[0], "research Q1");
    assert_eq!(a[0], "analyze Q2");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Synthesized"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_orchestrator_synthesizes_results() {
    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![
            tool_call("route_to_a", r#"{"task": "find alpha"}"#),
            tool_call("route_to_b", r#"{"task": "find beta"}"#),
        ]),
        Message::assistant("Combined: alpha and beta together"),
    ]);

    let a = MockAgent::new("a", "Agent A", "alpha is the first letter");
    let b = MockAgent::new("b", "Agent B", "beta is the second letter");
    let orchestrator = OrchestratorAgent::builder("orchestrator", model)
        .add_agent(a.clone() as Arc<dyn Agent>)
        .add_agent(b.clone() as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("What are alpha and beta?")],
        enable_streaming: false,
    };
    let result = orchestrator.run(input).await.expect("run failed");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(
            s.contains("Combined"),
            "synthesis should be in final answer, got: {}",
            s
        );
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_orchestrator_handles_subagent_failure() {
    struct FailingAgent {
        name: String,
        description: String,
    }
    #[async_trait]
    impl Agent for FailingAgent {
        fn name(&self) -> String {
            self.name.clone()
        }
        fn description(&self) -> String {
            self.description.clone()
        }
        async fn run(&self, _input: AgentInput) -> Result<AgentEvent, AgentError> {
            Err(AgentError::Agent("sub-agent failed".to_string()))
        }
        async fn resume(
            &self,
            _checkpoint: Vec<u8>,
            _info: ResumeInfo,
        ) -> Result<AgentEvent, AgentError> {
            unimplemented!()
        }
    }

    let model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call(
            "route_to_failing",
            r#"{"task": "do X"}"#,
        )]),
        Message::assistant("Partial result: sub-agent failed"),
    ]);

    let failing: Arc<dyn Agent> = Arc::new(FailingAgent {
        name: "failing".into(),
        description: "Always fails".into(),
    });
    let orchestrator = OrchestratorAgent::builder("orchestrator", model)
        .add_agent(failing)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Try")],
        enable_streaming: false,
    };
    let result = orchestrator.run(input).await.expect("run should not fail");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Partial"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}

#[tokio::test]
async fn test_orchestrator_nested() {
    let inner_model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_sub", r#"{"task": "inner Q"}"#)]),
        Message::assistant("Inner result"),
    ]);

    let outer_model = MockChatModel::new(vec![
        message_with_tool_calls(vec![tool_call("route_to_inner", r#"{"task": "outer Q"}"#)]),
        Message::assistant("Outer synthesized"),
    ]);

    let sub = MockAgent::new("sub", "Sub agent", "Sub result");
    let inner = OrchestratorAgent::builder("inner", inner_model)
        .add_agent(sub.clone() as Arc<dyn Agent>)
        .build();

    let outer = OrchestratorAgent::builder("outer", outer_model)
        .add_agent(Arc::new(inner) as Arc<dyn Agent>)
        .build();

    let input = AgentInput {
        messages: vec![Message::user("Nested")],
        enable_streaming: false,
    };
    let result = outer.run(input).await.expect("run failed");

    let received = sub.received_tasks.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "inner Q");

    let msg = result.output.message.expect("expected message");
    if let Content::Text(s) = msg.content {
        assert!(s.contains("Outer synthesized"), "got: {}", s);
    } else {
        panic!("expected text");
    }
}
