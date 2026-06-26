//! LLM Client 高层 API（Neuron / OpenAI / Anthropic 兼容）
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use synapse::llm::{create_default_http_client, OpenAIClient};
//! use synapse::components::model::BaseChatModel;
//! use synapse::schema::Message;
//!
//! # async fn run() {
//! let http = create_default_http_client();
//! let client = OpenAIClient::new(http, "your-api-key");
//!
//! // For Neuron (LLM runtime): set base_url
//! let client = client.with_base_url("http://neuron-host:8080/v1");
//!
//! let response = client.generate(
//!     vec![Message::user("Hello")],
//!     Default::default()
//! ).await.unwrap();
//! # }
//! ```

use std::sync::Arc;
use crate::transport::{HttpClient, DefaultHttpClient};

pub mod openai;
pub mod anthropic;

pub use openai::OpenAIClient;
pub use anthropic::AnthropicClient;

pub fn create_default_http_client() -> Arc<dyn HttpClient> {
    Arc::new(DefaultHttpClient::new())
}
