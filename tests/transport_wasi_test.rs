#![cfg(target_arch = "wasm32")]

use synapse::transport::{DefaultHttpClient, HttpClient, HttpRequest, HttpMethod, HttpResponse};

#[no_mangle]
pub extern "C" fn test_wasi_http_client_construction() {
    let _client: Box<dyn HttpClient> = Box::new(DefaultHttpClient::new());
}

#[no_mangle]
pub extern "C" fn test_wasi_http_request_shape() {
    let req = HttpRequest {
        method: HttpMethod::Get,
        url: "https://example.com".to_string(),
        headers: vec![],
        body: None,
    };
    assert_eq!(req.url, "https://example.com");
}
