use synapse::components::embedding::openai::OpenAIEmbedding;
use synapse::components::embedding::Embedding;
use synapse::llm::create_default_http_client;

#[tokio::test]
async fn test_openai_embedding_construction() {
    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "test-key");
    assert_eq!(emb.dimensions(), 1536);
}

#[tokio::test]
async fn test_openai_embedding_with_model() {
    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "test-key").with_model("text-embedding-3-large");
    assert_eq!(emb.dimensions(), 3072);
}

#[tokio::test]
async fn test_openai_embedding_with_base_url() {
    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "test-key").with_base_url("http://neuron-host:8080/v1");
    let _ = emb.dimensions();
}

#[tokio::test]
async fn test_openai_embed_with_mock_server() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/embeddings")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "object": "list",
                "data": [
                    {"object": "embedding", "index": 0, "embedding": [0.1, 0.2, 0.3]},
                    {"object": "embedding", "index": 1, "embedding": [0.4, 0.5, 0.6]}
                ],
                "model": "text-embedding-3-small",
                "usage": {"prompt_tokens": 5, "total_tokens": 5}
            }"#,
        )
        .create_async()
        .await;

    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "test-key").with_base_url(server.url());

    let result = emb
        .embed(vec!["hello".to_string(), "world".to_string()])
        .await;
    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    let vectors = result.unwrap();
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0], vec![0.1, 0.2, 0.3]);
    assert_eq!(vectors[1], vec![0.4, 0.5, 0.6]);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_openai_embed_empty_input() {
    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "test-key");
    let result = emb.embed(vec![]).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_openai_embed_error_handling() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/embeddings")
        .with_status(401)
        .with_body(r#"{"error": {"message": "invalid api key"}}"#)
        .create_async()
        .await;

    let http = create_default_http_client();
    let emb = OpenAIEmbedding::new(http, "bad-key").with_base_url(server.url());

    let result = emb.embed(vec!["test".to_string()]).await;
    assert!(result.is_err());
    mock.assert_async().await;
}
