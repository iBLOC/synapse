use crate::transport::HttpClient;
use std::sync::Arc;

pub struct AnthropicClient {
    _http: Arc<dyn HttpClient>,
}

impl AnthropicClient {
    pub fn new(http: Arc<dyn HttpClient>, api_key: impl Into<String>) -> Self {
        Self { _http: http }
    }
}
