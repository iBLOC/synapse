use synapse::transport::{DefaultHttpClient, HttpClient, HttpRequest, HttpMethod, HttpResponse};
use std::sync::Arc;

#[tokio::test]
async fn test_default_http_client_construction() {
    let client: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());
    let req = HttpRequest {
        method: HttpMethod::Get,
        url: "https://example.com".to_string(),
        headers: vec![],
        body: None,
    };
    let _ = client.execute(req).await;
}

#[tokio::test]
async fn test_post_json_with_mock_server() {
    let mut server = mockito::Server::new_async().await;
    let mock = server.mock("POST", "/test")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "hello"}"#)
        .create_async()
        .await;

    let url = format!("{}/test", server.url());
    let client: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());

    #[derive(serde::Serialize)]
    struct ReqBody { name: String }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct RespBody { message: String }

    let result: Result<RespBody, _> = client.post_json(&url, &ReqBody { name: "test".into() }).await;
    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    let resp = result.unwrap();
    assert_eq!(resp.message, "hello");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_status_error_handling() {
    let mut server = mockito::Server::new_async().await;
    let mock = server.mock("GET", "/missing")
        .with_status(404)
        .with_body("Not Found")
        .create_async()
        .await;

    let url = format!("{}/missing", server.url());
    let client: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());

    let req = HttpRequest {
        method: HttpMethod::Get,
        url,
        headers: vec![],
        body: None,
    };
    let result = client.execute(req).await;
    assert!(result.is_ok(), "execute should return Ok response with 404 status");
    let resp: HttpResponse = result.unwrap();
    assert_eq!(resp.status, 404);
    mock.assert_async().await;
}

#[tokio::test]
async fn test_post_json_with_headers() {
    let mut server = mockito::Server::new_async().await;
    let mock = server.mock("POST", "/auth")
        .match_header("authorization", "Bearer test-token")
        .with_status(200)
        .with_body(r#"{"ok": true}"#)
        .create_async()
        .await;

    let url = format!("{}/auth", server.url());
    let client: Arc<dyn HttpClient> = Arc::new(DefaultHttpClient::new());

    #[derive(serde::Serialize)]
    struct ReqBody { data: String }

    #[derive(serde::Deserialize, Debug)]
    struct RespBody { ok: bool }

    let result: Result<RespBody, _> = client.post_json_with_headers(
        &url,
        &[("Authorization", "Bearer test-token")],
        &ReqBody { data: "x".into() }
    ).await;
    assert!(result.is_ok(), "Expected Ok but got: {:?}", result);
    assert!(result.unwrap().ok);
    mock.assert_async().await;
}
