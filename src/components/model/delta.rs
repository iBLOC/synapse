//! Streaming delta types for LLM provider responses.
//!
//! [`MessageDelta`] is the unit of progress a provider yields mid-stream: a
//! patch to the assistant message (text + tool-call deltas + finish reason +
//! usage). Higher layers — [`crate::adk::stream::AgentStreamEvent`] and the
//! A2A wire layer — translate these into their richer event vocabularies.

/// A single token/tool-call update yielded by an LLM provider stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageDelta {
    /// Incremental text to append to the assistant message content.
    pub content_delta: String,
    /// Incremental updates to in-flight tool calls, keyed by `index`.
    pub tool_call_deltas: Vec<ToolCallDelta>,
    /// Set on the terminal delta (after `[DONE]` or equivalent).
    pub finish_reason: Option<FinishReason>,
    /// Provider-reported usage; only present on terminal delta when
    /// `stream_options.include_usage = true` (or provider equivalent).
    pub usage: Option<Usage>,
}

/// One slot in a tool-call update; multiple slots may arrive in a single
/// delta (parallel tool calls).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallDelta {
    /// Stable index in the parallel tool-call array.
    pub index: usize,
    /// Set on the first delta for this slot.
    pub id: Option<String>,
    /// Set on the first delta for this slot (Anthropic's `content_block_start`).
    pub name: Option<String>,
    /// JSON arguments fragment; concatenate across deltas to reconstruct.
    pub arguments_delta: String,
}

/// Why the provider stopped producing tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// Natural stop.
    Stop,
    /// Provider wants the caller to execute one or more tool calls.
    ToolCalls,
    /// Hit `max_tokens` before natural stop.
    Length,
    /// Provider-side content filter triggered.
    ContentFilter,
    /// Provider-side hard error (auth, 5xx, schema).
    Error,
}

/// Token usage reported by the provider.
///
/// `cached_input_tokens` is provider-agnostic (OpenAI cached, Anthropic
/// `cache_read_input_tokens`); we normalize to a single field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_input_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_default_is_empty() {
        let d = MessageDelta::default();
        assert!(d.content_delta.is_empty());
        assert!(d.tool_call_deltas.is_empty());
        assert!(d.finish_reason.is_none());
        assert!(d.usage.is_none());
    }

    #[test]
    fn finish_reason_distinct_variants() {
        // A simple exhaustiveness sanity check; the compiler enforces this,
        // but the test documents intent.
        let _rounds = [
            FinishReason::Stop,
            FinishReason::ToolCalls,
            FinishReason::Length,
            FinishReason::ContentFilter,
            FinishReason::Error,
        ];
    }
}
