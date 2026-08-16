use std::sync::Arc;
use synapse::components::model::{BaseChatModel, GenerateOptions};
use synapse::llm::{create_default_http_client, AnthropicClient, OpenAIClient};
use synapse::schema::{Content, Message, RoleType};

#[tokio::test]
async fn test_openai_client_construction() {
    let http = create_default_http_client();
    let client = OpenAIClient::new(http, "test-api-key");
    let _model: &dyn BaseChatModel = &client;
}

#[tokio::test]
async fn test_openai_client_with_base_url() {
    let http = create_default_http_client();
    let client = OpenAIClient::new(http, "test-key").with_base_url("http://neuron-host:8080/v1");
    let _model: &dyn BaseChatModel = &client;
}

#[tokio::test]
async fn test_anthropic_client_construction() {
    let http = create_default_http_client();
    let _client = AnthropicClient::new(http, "test-key");
}

#[tokio::test]
async fn test_create_default_http_client() {
    let http1 = create_default_http_client();
    let http2 = create_default_http_client();
    assert!(Arc::strong_count(&http1) >= 1);
    assert!(Arc::strong_count(&http2) >= 1);
}

#[tokio::test]
async fn test_openai_generate_with_mock_server() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello from mock!"
                }
            }]
        }"#,
        )
        .create_async()
        .await;

    let http = create_default_http_client();
    let client = OpenAIClient::new(http, "test-key").with_base_url(server.url());

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
    if let Content::Text(s) = msg.content {
        assert_eq!(s, "Hello from mock!");
    } else {
        panic!("Expected Text content");
    }
    mock.assert_async().await;
}
