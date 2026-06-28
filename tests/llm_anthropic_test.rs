use std::sync::Arc;
use synapse::components::model::{BaseChatModel, GenerateOptions};
use synapse::llm::{create_default_http_client, AnthropicClient};
use synapse::schema::{Content, Message, RoleType};

#[tokio::test]
async fn test_anthropic_client_construction() {
    let http = create_default_http_client();
    let _client = AnthropicClient::new(http, "test-key");
}

#[tokio::test]
async fn test_anthropic_client_with_base_url() {
    let http = create_default_http_client();
    let _client =
        AnthropicClient::new(http, "test-key").with_base_url("https://custom.anthropic.com/v1");
}

#[tokio::test]
async fn test_anthropic_client_with_model() {
    let http = create_default_http_client();
    let _client = AnthropicClient::new(http, "test-key").with_model("claude-3-5-haiku-20241022");
}

#[tokio::test]
async fn test_anthropic_generate_with_mock_server() {
    use mockito::Server;
    use synapse::components::model::GenerateOptions;

    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .match_header("x-api-key", "test-key")
        .match_header("anthropic-version", "2023-06-01")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": "msg_01",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello from mock!"}],
                "stop_reason": "end_turn",
                "model": "claude-3-5-sonnet-20241022"
            }"#,
        )
        .create_async()
        .await;

    let http = create_default_http_client();
    let client = AnthropicClient::new(http, "test-key").with_base_url(server.url());

    let result = client
        .generate(
            vec![Message {
                role: RoleType::User,
                content: Content::Text("Hi".to_string()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            }],
            GenerateOptions::default(),
        )
        .await;

    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    let msg = result.unwrap();
    match msg.content {
        Content::Text(s) => assert_eq!(s, "Hello from mock!"),
        _ => panic!("Expected Text content"),
    }
    mock.assert_async().await;
}

#[tokio::test]
async fn test_anthropic_generate_with_system_message() {
    use mockito::Server;
    use synapse::components::model::GenerateOptions;

    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_body(
            r#"{
                "id": "msg_02",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "I am a helpful assistant."}],
                "stop_reason": "end_turn"
            }"#,
        )
        .create_async()
        .await;

    let http = create_default_http_client();
    let client = AnthropicClient::new(http, "test-key").with_base_url(server.url());

    let result = client
        .generate(
            vec![
                Message {
                    role: RoleType::System,
                    content: Content::Text("You are a helpful assistant.".to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                Message {
                    role: RoleType::User,
                    content: Content::Text("Hi".to_string()),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            ],
            GenerateOptions::default(),
        )
        .await;

    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_anthropic_generate_error_handling() {
    use mockito::Server;
    use synapse::components::model::GenerateOptions;

    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(401)
        .with_body(r#"{"error": {"type": "authentication_error", "message": "invalid x-api-key"}}"#)
        .create_async()
        .await;

    let http = create_default_http_client();
    let client = AnthropicClient::new(http, "bad-key").with_base_url(server.url());

    let result = client
        .generate(
            vec![Message {
                role: RoleType::User,
                content: Content::Text("Hi".to_string()),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            }],
            GenerateOptions::default(),
        )
        .await;

    assert!(result.is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_create_default_http_client() {
    let http = create_default_http_client();
    let _client = AnthropicClient::new(http, "test-key");
}
