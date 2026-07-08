use synapse::llm::create_default_http_client;
use synapse::protocol::{
    A2AAdapter, AgentAdapter, AgentEndpoint, AgentProtocol, AgentRequest, ProtocolMessage,
};

#[tokio::test]
async fn test_a2a_adapter_new_with_http() {
    let http = create_default_http_client();
    let _adapter = A2AAdapter::new(http);
}

#[tokio::test]
async fn test_a2a_adapter_new_default() {
    let _adapter = A2AAdapter::new_default();
}

#[tokio::test]
async fn test_a2a_send_task_with_mock() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "id": "task-1",
                "status": "completed",
                "messages": [
                    {"role": "agent", "parts": [{"type": "text", "text": "Hi back!"}]}
                ],
                "result": null
            }
        }"#,
        )
        .create_async()
        .await;

    let http = create_default_http_client();
    let adapter = A2AAdapter::new(http);

    let endpoint = AgentEndpoint {
        url: server.url(),
        protocol: AgentProtocol::A2A,
        capabilities: vec![],
        metadata: serde_json::Value::Null,
    };

    let request = AgentRequest {
        task_id: "task-1".to_string(),
        messages: vec![ProtocolMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            metadata: serde_json::Value::Null,
        }],
        context: serde_json::Value::Null,
    };

    let result: Result<synapse::protocol::AgentResponse, String> =
        adapter.send_task(&endpoint, &request).await;
    assert!(result.is_ok(), "send_task failed: {:?}", result);
    let resp = result.unwrap();
    assert_eq!(resp.task_id, "task-1");
    assert_eq!(resp.messages.len(), 1);
    mock.assert_async().await;
}
