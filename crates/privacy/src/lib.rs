//! Outbound privacy: PII redaction for external (cloud) model calls.
//!
//! ## Why a standalone crate
//!
//! The mechanism is shared by every outbound LLM stack on the platform:
//! `synapse` (re-exported as `synapse::privacy`), `neuron`'s remote
//! forwarder (the desktop's cloud-model path) and `prefrontal`'s agent
//! providers. It lives in its own crate so those consumers can take the
//! engine without pulling `synapse`'s heavy runtime dependencies
//! (wasmtime, a2a, axum) — `neuron` in particular must stay lean.
//!
//! Wiring happens at each consumer's provider boundary; policy (which
//! provider redacts what, user-facing toggles) stays in the product
//! repos (encephalon provider config + desktop UI).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use synapse_privacy::{RedactionEngine, RegexPiiDetector};
//! use std::sync::Arc;
//!
//! let engine = RedactionEngine::new(Arc::new(RegexPiiDetector::new()));
//! let outcome = engine.redact("联系 13800138000");
//! // outcome.text == "联系 [PHONE_1]"
//! let restored = engine.restore(&outcome.text);
//! // restored == "联系 13800138000"
//! ```
//!
//! ## Design constraints
//!
//! - **Reversible pseudonymization**, not `[REDACTED]`: translation /
//!   summary tasks need the model to reason about shape-consistent
//!   placeholders. The mapping table lives in memory only and never
//!   leaves the process — persisting it would create a new leak target.
//! - **Redact on send, restore on receive**: both at the HTTP boundary,
//!   so session stores / tool arguments / agent loops keep seeing real
//!   content and no persistence layer needs to change.
//! - **Consistent mapping across calls**: agent loops resend full
//!   message history every turn; the same engine instance (per
//!   provider) maps the same raw value to the same token so history
//!   stays stable.
//! - Reports never contain matched raw text.

mod classifier;
mod detector;
mod pseudonym;

pub use classifier::is_external_url;
pub use detector::{PiiCategory, PiiDetector, RegexPiiDetector, Span};
pub use pseudonym::PseudonymMapper;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One category-count entry in a [`RedactionReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionRecord {
    pub category: PiiCategory,
    pub count: usize,
}

/// Per-call redaction summary. Contains counts only — never the
/// matched raw text (otherwise the report itself becomes a leak
/// surface when logged).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionReport {
    pub total: usize,
    pub by_category: Vec<RedactionRecord>,
}

/// Result of a single `redact` pass.
#[derive(Debug, Clone)]
pub struct RedactionOutcome {
    /// Pseudonymized text, safe to send to an external provider.
    pub text: String,
    pub report: RedactionReport,
}

/// Redaction engine: detect → pseudonymize on the way out, restore on
/// the way back. One engine per provider instance keeps pseudonyms
/// consistent across the turns of a conversation.
///
/// Thread-safe: the pseudonym map is guarded internally; the detector
/// is immutable after construction.
pub struct RedactionEngine {
    detector: Arc<dyn PiiDetector>,
    mapper: PseudonymMapper,
}

impl RedactionEngine {
    pub fn new(detector: Arc<dyn PiiDetector>) -> Self {
        Self {
            detector,
            mapper: PseudonymMapper::new(),
        }
    }

    /// Convenience constructor with the default rule-based detector.
    pub fn with_regex_detector() -> Self {
        Self::new(Arc::new(RegexPiiDetector::new()))
    }

    /// Replace every detected PII span in `text` with a stable
    /// pseudonym token. Spans are assumed char-aligned (byte offsets
    /// from regex matches always are).
    pub fn redact(&self, text: &str) -> RedactionOutcome {
        let spans = self.detector.detect(text);
        let mut counts: BTreeMap<PiiCategory, usize> = BTreeMap::new();
        let mut out = text.to_string();
        // Replace right-to-left so earlier byte offsets stay valid.
        for span in spans.iter().rev() {
            let raw = &text[span.start..span.end];
            let token = self.mapper.pseudonym_for(span.category, raw);
            out.replace_range(span.start..span.end, &token);
            *counts.entry(span.category).or_insert(0) += 1;
        }
        let by_category: Vec<RedactionRecord> = counts
            .into_iter()
            .map(|(category, count)| RedactionRecord { category, count })
            .collect();
        let total: usize = by_category.iter().map(|r| r.count).sum();
        RedactionOutcome {
            text: out,
            report: RedactionReport { total, by_category },
        }
    }

    /// Restore pseudonym tokens back to the original values. Unknown
    /// tokens (e.g. the model mangled a placeholder) are left as-is —
    /// graceful degradation, not an error.
    pub fn restore(&self, text: &str) -> String {
        self.mapper.restore(text)
    }

    /// Number of distinct raw values currently mapped. Useful as a
    /// growth guard: engines are per-provider and process-lifetime.
    pub fn mapped_values(&self) -> usize {
        self.mapper.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_and_restore_round_trip() {
        let engine = RedactionEngine::with_regex_detector();
        let input = "张三的手机是 13800138000，邮箱 zhangsan@example.com";
        let outcome = engine.redact(input);
        assert!(outcome.text.contains("[PHONE_1]"), "got {}", outcome.text);
        assert!(
            outcome.text.contains("[EMAIL_1]"),
            "got {}",
            outcome.text
        );
        assert!(!outcome.text.contains("13800138000"));
        assert!(!outcome.text.contains("zhangsan@example.com"));
        assert_eq!(engine.restore(&outcome.text), input);
    }

    #[test]
    fn tokens_stable_across_calls() {
        let engine = RedactionEngine::with_regex_detector();
        let first = engine.redact("电话 13800138000");
        let second = engine.redact("再拨 13800138000 一次");
        assert_eq!(first.text, "电话 [PHONE_1]");
        assert_eq!(second.text, "再拨 [PHONE_1] 一次");
    }

    #[test]
    fn already_redacted_text_is_stable() {
        let engine = RedactionEngine::with_regex_detector();
        let once = engine.redact("号码 13800138000");
        let twice = engine.redact(&once.text);
        assert_eq!(once.text, twice.text);
        assert_eq!(twice.report.total, 0);
    }

    #[test]
    fn report_counts_without_raw_text() {
        let engine = RedactionEngine::with_regex_detector();
        let outcome = engine.redact("13800138000 和 13900139000");
        assert_eq!(outcome.report.total, 2);
        assert_eq!(
            outcome.report.by_category,
            vec![RedactionRecord {
                category: PiiCategory::Phone,
                count: 2
            }]
        );
        let debug = format!("{:?}", outcome.report);
        assert!(!debug.contains("13800138000"));
    }

    #[test]
    fn clean_text_passes_through_untouched() {
        let engine = RedactionEngine::with_regex_detector();
        let input = "今天天气不错，去公园走了走。";
        let outcome = engine.redact(input);
        assert_eq!(outcome.text, input);
        assert_eq!(outcome.report.total, 0);
        assert_eq!(engine.mapped_values(), 0);
    }

    #[test]
    fn multi_byte_offsets_stay_char_aligned() {
        let engine = RedactionEngine::with_regex_detector();
        // CJK before + emoji after the PII to force non-ASCII byte offsets.
        let input = "信号📶强，卡号 4111111111111111 优👍";
        let outcome = engine.redact(input);
        assert!(outcome.text.contains("[BANK_CARD_1]"), "got {}", outcome.text);
        assert_eq!(engine.restore(&outcome.text), input);
    }
}
