use std::sync::Arc;
use crate::transport::{HttpClient, DefaultHttpClient};

pub mod openai;
pub mod anthropic;

pub use openai::OpenAIClient;
pub use anthropic::AnthropicClient;

pub fn create_default_http_client() -> Arc<dyn HttpClient> {
    Arc::new(DefaultHttpClient::new())
}
