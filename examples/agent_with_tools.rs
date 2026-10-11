//! Give an agent a custom tool: define a `Calculator` implementing
//! `BaseTool` + `InvokableTool`, bundle it in a `ToolsNode`, attach the
//! node to a GraphReActAgent, and let the ReAct loop decide when to call.
//!
//! Run:
//!   OPENAI_API_KEY=sk-... cargo run --example agent_with_tools

use std::sync::Arc;

use async_trait::async_trait;
use synapse::adk::agent::react::GraphReActAgent;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::components::model::OpenAIModel;
use synapse::components::tool::{BaseTool, InvokableTool, ToolError, ToolResult_};
use synapse::compose::tool_node::{ToolsNode, ToolsNodeConfig};
use synapse::llm::create_default_http_client;
use synapse::schema::{Message, ToolInfo};

struct Calculator;

#[async_trait]
impl BaseTool for Calculator {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send {
        async move {
            Ok(ToolInfo {
                name: "calculator".into(),
                description: "Evaluates a basic arithmetic expression.".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": {"type": "string"}
                    },
                    "required": ["expression"]
                }),
            })
        }
    }
}

#[async_trait]
impl InvokableTool for Calculator {
    async fn invoke(&self, _arguments: String) -> ToolResult_ {
        // A real implementation would parse and evaluate the expression.
        Ok("42".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = create_default_http_client();
    let model = Arc::new(OpenAIModel::new(http, "sk-..."));

    let tools_node = Arc::new(ToolsNode::new(ToolsNodeConfig {
        tools: vec![Arc::new(Calculator)],
        ..Default::default()
    })?);

    let agent = GraphReActAgent::builder()
        .name("math-agent")
        .description("Answers arithmetic questions with the calculator tool")
        .with_model(model)
        .with_tools_node(tools_node)
        .max_steps(5)
        .build()?;

    let input = AgentInput {
        messages: vec![Message::user("What is 5 + 3?")],
        enable_streaming: false,
    };

    let event = agent.run(input).await?;
    // The agent will: see the question, call `calculator`, and answer.
    if let Some(msg) = event.output.message {
        println!("Response: {:?}", msg.content);
    }

    Ok(())
}
