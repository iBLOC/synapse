use synapse::components::model::{OpenAIModel, BaseChatModel, GenerateOptions};
use synapse::llm::create_default_http_client;
use synapse::schema::{Message, RoleType, Content};

#[tokio::test]
async fn test_openai_model_construction() {
    let http = create_default_http_client();
    let _model = OpenAIModel::new(http, "test-key");
}

#[tokio::test]
async fn test_openai_model_generate_with_mock() {
    let mut server = mockito::Server::new_async().await;
    let mock = server.mock("POST", "/chat/completions")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "choices": [{
                "message": {"role": "assistant", "content": "OK"}
            }]
        }"#)
        .create_async()
        .await;

    let http = create_default_http_client();
    let model = OpenAIModel::new(http, "test-key")
        .with_base_url(server.url());

    let result: Result<Message, synapse::components::model::ModelError> = model.generate(
        vec![Message {
            role: RoleType::User,
            content: Content::Text("Hi".to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }],
        GenerateOptions::default()
    ).await;
    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    mock.assert_async().await;
}
