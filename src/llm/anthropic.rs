use std::sync::Arc;
use crate::transport::HttpClient;

pub struct AnthropicClient {
    _http: Arc<dyn HttpClient>,
}

impl AnthropicClient {
    pub fn new(http: Arc<dyn HttpClient>, api_key: impl Into<String>) -> Self {
        Self { _http: http }
    }
}
