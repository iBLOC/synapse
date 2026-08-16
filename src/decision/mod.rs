//! `synapse::decision` — first-class decision tracking for agent lobes.
//!
//! Inspired by Semantica's `DecisionRecorder` + `PolicyEngine` + `CausalChainAnalyzer`,
//! but designed to be **storage-agnostic** and **lobe-friendly**:
//!
//! - **Lobe-agnostic** — any agent framework (cerebral-cortex, tiangong-agent,
//!   paperclip-style orchestrators) can `ProviderSet::with_decisions(...)` and
//!   wrap their `chat()` / `tool_use()` calls with `decision!(..) {}`.
//! - **Storage-agnostic** — the default `HippocampusDecisionStore` persists
//!   decisions as graph nodes (via the cortex `LanceGraphStorage` Cypher backend),
//!   but custom stores (Redis, Postgres, S3) can be plugged via
//!   `DecisionStore` trait.
//! - **Editable / auditable** — every record is a first-class `Decision` struct
//!   that can be retrieved, edited (`DecisionEdit { .. }`), and traced
//!   (`causal_chain(direction)`).
//! - **Policy-gated** — `PolicyEngine::check_compliance(decision, ...)` returns
//!   `Compliant | Violation { reason }` *before* the decision is recorded.
//!
//! See [`docs/decision-tracking.md`](https://docs.shadow/dev/synapse/decision) for the
//! design rationale and a worked example.

#![allow(ambiguous_glob_reexports)]

pub mod decision;
pub mod policy;
pub mod store;
pub mod causal;

pub use decision::{Decision, DecisionBuilder, DecisionCategory, DecisionMaker, DecisionOutcome};
pub use policy::{Policy, PolicyEngine, ComplianceResult, PolicyVersion};
pub use store::{
    DecisionStore, DecisionStoreError, DecisionStoreResult, DecisionQuery,
    HippocampusConfig, HippocampusDecisionStore, InMemoryDecisionStore,
};
pub use causal::{CausalChain, CausalDirection};
