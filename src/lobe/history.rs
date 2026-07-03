//! `HistorySummarizer` — appends turns and triggers LLM summarization on overflow.
//!
//! Stateless: each call takes a prev summary + new turn, returns an
//! updated summary. Caller (cortex) persists the result.

use std::sync::Arc;

use crate::providers::{ChatOptions, Content, LLMProvider, Message, Role};

pub struct HistorySummarizer {
    llm: Arc<dyn LLMProvider>,
    max_tokens: usize,
}

impl HistorySummarizer {
    pub fn new(llm: Arc<dyn LLMProvider>, max_tokens: usize) -> Self {
        Self { llm, max_tokens }
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    pub fn estimate_tokens(s: &str) -> usize {
        s.len().div_ceil(4)
    }

    /// Build raw history string. If total tokens > max_tokens, call LLM to summarize.
    pub async fn append_turn(
        &self,
        prev_summary: Option<&str>,
        user_turn: &str,
        assistant_turn: &str,
    ) -> Result<String, String> {
        let raw = match prev_summary {
            Some(p) if !p.is_empty() => {
                format!("{p}\n[User]: {user_turn}\n[Assistant]: {assistant_turn}")
            }
            _ => format!("[User]: {user_turn}\n[Assistant]: {assistant_turn}"),
        };

        if Self::estimate_tokens(&raw) <= self.max_tokens {
            return Ok(raw);
        }

        let prompt = format!(
            "Summarize the following conversation in 500 words or less, preserving key intent and facts:\n\n{raw}"
        );
        let messages = vec![Message {
            role: Role::User,
            content: Content::text(prompt),
        }];
        let options = ChatOptions {
            model: None,
            temperature: Some(0.3),
            max_tokens: Some(1000),
            ..Default::default()
        };
        let response = self.llm.chat(messages, options).await
            .map_err(|e| format!("summarize: {e}"))?;
        Ok(response.content.text.unwrap_or_default())
    }
}
