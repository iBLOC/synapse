//! Outbound privacy: PII redaction for external (cloud) model calls.
//!
//! The engine lives in the `synapse-privacy` crate (same repo,
//! `crates/privacy`) so lean consumers — notably `neuron`, whose remote
//! forwarder is the desktop's actual cloud-model path — can depend on
//! it without pulling synapse's runtime dependencies (wasmtime, a2a,
//! axum). This module is a re-export, so `synapse::privacy::*` stays
//! the canonical import path inside the synapse ecosystem.
//!
//! ## Layering
//!
//! Mechanism only — the detector, pseudonym mapper and URL classifier
//! live here so every outbound LLM stack can share them. Wiring happens
//! at the provider boundary of each consumer (prefrontal wraps its
//! external `LlmProvider` implementations; neuron wraps its remote
//! forwarder); policy (which provider redacts what, user-facing
//! toggles) stays in the product repos.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use synapse::privacy::{RedactionEngine, RegexPiiDetector};
//! use std::sync::Arc;
//!
//! let engine = RedactionEngine::new(Arc::new(RegexPiiDetector::new()));
//! let outcome = engine.redact("联系 13800138000");
//! // outcome.text == "联系 [PHONE_1]"
//! let restored = engine.restore(&outcome.text);
//! // restored == "联系 13800138000"
//! ```

pub use synapse_privacy::{
    is_external_url, PiiCategory, PiiDetector, PseudonymMapper, RedactionEngine,
    RedactionOutcome, RedactionRecord, RedactionReport, RegexPiiDetector, Span,
};
