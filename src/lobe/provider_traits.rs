//! Re-exports of existing provider traits + new `EmotionProvider`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use crate::providers::LLMProvider;

/// Input to emotion detection (text segment or structured data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionInput {
    pub text: String,
    #[serde(default)]
    pub context: Option<String>,
}

/// Emotion detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionOutput {
    /// Dominant emotion label: "happy", "sad", "angry", "neutral", etc.
    pub emotion: String,
    /// Confidence 0.0–1.0.
    pub confidence: f32,
}

/// Detect emotion from text input.
/// Lobe-specific impls can use an LLM, on-device classifier, or cloud API.
#[async_trait]
pub trait EmotionProvider: Send + Sync {
    async fn detect(&self, input: &EmotionInput) -> Result<EmotionOutput, String>;
}
