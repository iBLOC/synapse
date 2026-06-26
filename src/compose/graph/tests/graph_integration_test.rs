use crate::compose::graph::{Chain, CompiledGraph, Graph, NodeTriggerMode, Workflow, END, START};
use crate::compose::runnable::{
    invokable_lambda, Context, Runnable, RunnableError, RunnableFuture, RunnableResult,
};
use std::sync::Arc;

fn upper_lambda() -> Arc<dyn Runnable<serde_json::Value, serde_json::Value>> {
    invokable_lambda(
        "upper",
        |_ctx: Context, input: serde_json::Value| async move {
            let s = input.as_str().unwrap_or("");
            Ok(serde_json::Value::String(s.to_uppercase()))
        },
    )
}

fn add_prefix_lambda(
    prefix: &'static str,
) -> Arc<dyn Runnable<serde_json::Value, serde_json::Value>> {
    invokable_lambda(
        "add_prefix",
        move |_ctx: Context, input: serde_json::Value| async move {
            let s = input.as_str().unwrap_or("");
            Ok(serde_json::Value::String(format!("{}{}", prefix, s)))
        },
    )
}

#[tokio::test]
async fn test_graph_linear_chain() {
    let mut graph: Graph<serde_json::Value, serde_json::Value> =
        Graph::new(NodeTriggerMode::AllPredecessor);
    graph.add_node("upper", upper_lambda());
    graph.add_edge(START, "upper");
    graph.add_edge("upper", END);

    let compiled = graph.compile().unwrap();
    let ctx = Context::new();
    let result = compiled
        .invoke(ctx, serde_json::json!("hello"))
        .await
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "HELLO");
}

#[tokio::test]
async fn test_graph_two_nodes() {
    let mut graph: Graph<serde_json::Value, serde_json::Value> =
        Graph::new(NodeTriggerMode::AllPredecessor);
    graph.add_node("prefix", add_prefix_lambda("PREFIX_"));
    graph.add_node("upper", upper_lambda());
    graph.add_edge(START, "prefix");
    graph.add_edge("prefix", "upper");
    graph.add_edge("upper", END);

    let compiled = graph.compile().unwrap();
    let ctx = Context::new();
    let result = compiled
        .invoke(ctx, serde_json::json!("hello"))
        .await
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "PREFIX_HELLO");
}

#[tokio::test]
async fn test_graph_max_steps_exceeded() {
    let mut graph: Graph<serde_json::Value, serde_json::Value> =
        Graph::new(NodeTriggerMode::AllPredecessor);
    let node_a = invokable_lambda(
        "node_a",
        |_ctx: Context, input: serde_json::Value| async move { Ok(input) },
    );
    let node_b = invokable_lambda(
        "node_b",
        |_ctx: Context, input: serde_json::Value| async move { Ok(input) },
    );
    graph.add_node("node_a", node_a);
    graph.add_node("node_b", node_b);
    graph.add_edge(START, "node_a");
    graph.add_edge("node_a", "node_b");
    graph.add_edge("node_b", "node_a");
    graph.add_edge("node_a", END);

    let compiled = graph.compile().unwrap();
    let ctx = Context::new();
    let result = compiled.invoke(ctx, serde_json::json!("test")).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("exceeded max steps"));
}

#[tokio::test]
async fn test_workflow_builder() {
    let compiled = Workflow::<serde_json::Value, serde_json::Value>::new()
        .add_node("upper", upper_lambda())
        .add_edge(START, "upper")
        .add_edge("upper", END)
        .compile()
        .unwrap();

    let ctx = Context::new();
    let result = compiled
        .invoke(ctx, serde_json::json!("workflow"))
        .await
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "WORKFLOW");
}

#[tokio::test]
async fn test_chain_builder() {
    let compiled = Chain::<serde_json::Value>::new()
        .append_lambda(
            "upper",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("");
                Ok(serde_json::Value::String(s.to_uppercase()))
            },
        )
        .append_lambda(
            "bracket",
            |_ctx: Context, input: serde_json::Value| async move {
                let s = input.as_str().unwrap_or("");
                Ok(serde_json::Value::String(format!("[{}]", s)))
            },
        )
        .compile()
        .unwrap();

    let ctx = Context::new();
    let result = compiled
        .invoke(ctx, serde_json::json!("chain"))
        .await
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "[CHAIN]");
}

#[tokio::test]
async fn test_graph_no_nodes() {
    let mut graph: Graph<serde_json::Value, serde_json::Value> =
        Graph::new(NodeTriggerMode::AllPredecessor);
    graph.add_edge(START, END);
    let compiled = graph.compile().unwrap();
    let ctx = Context::new();
    let result = compiled.invoke(ctx, serde_json::json!("test")).await;
    assert!(result.is_ok());
}
