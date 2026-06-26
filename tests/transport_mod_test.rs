use synapse::transport::{HttpError, HttpMethod, HttpRequest, HttpResponse};

#[test]
fn test_http_request_builder() {
    let req = HttpRequest {
        method: HttpMethod::Post,
        url: "https://api.example.com/v1/chat".to_string(),
        headers: vec![("Content-Type".into(), "application/json".into())],
        body: Some(b"{}".to_vec()),
    };
    assert_eq!(req.url, "https://api.example.com/v1/chat");
    assert!(matches!(req.method, HttpMethod::Post));
}

#[test]
fn test_http_error_display() {
    let err = HttpError::Status(404, "Not Found".to_string());
    assert_eq!(err.to_string(), "status error: 404 Not Found");
}

#[test]
fn test_http_method_equality() {
    assert_eq!(HttpMethod::Get, HttpMethod::Get);
    assert_ne!(HttpMethod::Get, HttpMethod::Post);
}
