use synapse::llm::{create_default_http_client, OpenAIClient, AnthropicClient};
use synapse::components::model::BaseChatModel;
use std::sync::Arc;

#[tokio::test]
async fn test_openai_client_construction() {
    let http = create_default_http_client();
    let client = OpenAIClient::new(http, "test-api-key");
    let _model: &dyn BaseChatModel = &client;
}

#[tokio::test]
async fn test_openai_client_with_base_url() {
    let http = create_default_http_client();
    let client = OpenAIClient::new(http, "test-key")
        .with_base_url("http://neuron-host:8080/v1");
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
