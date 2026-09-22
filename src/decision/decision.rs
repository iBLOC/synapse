//! `Decision` data model + `DecisionBuilder` for lobe-side decision tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// High-level decision category. Lobe authors may extend with custom
/// `&'static str` values; the helpers below treat unknown values as
/// "other" without rejecting them (storage-agnostic).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCategory {
    Other(String),
    ModelSelection,
    ToolSelection,
    Routing,
    Approval,
    Escalation,
    Consolidation,
    MemoryWrite,
    MemorySupersede,
    TaskAssignment,
}

impl Default for DecisionCategory {
    fn default() -> Self {
        Self::Other("default".to_string())
    }
}

impl DecisionCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ModelSelection => "model_selection",
            Self::ToolSelection => "tool_selection",
            Self::Routing => "routing",
            Self::Approval => "approval",
            Self::Escalation => "escalation",
            Self::Consolidation => "consolidation",
            Self::MemoryWrite => "memory_write",
            Self::MemorySupersede => "memory_supersede",
            Self::TaskAssignment => "task_assignment",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// Who (or what subsystem) made the decision. In the Shadow stack this
/// maps to one of the four native lobes (prefrontal, temporal, parietal,
/// occipital) or an external agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DecisionMaker {
    Prefrontal,
    Temporal,
    Parietal,
    Occipital,
    EncephalonOrchestrator,
    External(String),
}

impl DecisionMaker {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Prefrontal => "prefrontal",
            Self::Temporal => "temporal",
            Self::Parietal => "parietal",
            Self::Occipital => "occipital",
            Self::EncephalonOrchestrator => "encephalon",
            Self::External(s) => s.as_str(),
        }
    }
}

/// Outcome of a decision. `Other(String)` is the escape hatch for
/// lobe-specific values that don't fit the canonical list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DecisionOutcome {
    Selected(String),
    Approved,
    Rejected,
    Deferred,
    Escalated,
    Superseded,
    Other(String),
}

impl DecisionOutcome {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Selected(s) => s.as_str(),
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Deferred => "deferred",
            Self::Escalated => "escalated",
            Self::Superseded => "superseded",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// First-class decision record. Aligns with Semantica's `Decision` dataclass
/// so cross-system migration stays feasible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub decision_id: Uuid,
    pub tenant_id: String,
    pub category: DecisionCategory,
    pub scenario: String,
    pub reasoning: String,
    pub outcome: DecisionOutcome,
    pub confidence: f32,
    pub decision_maker: DecisionMaker,
    pub entities: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub caused_by: Vec<Uuid>,
    pub causes: Vec<Uuid>,
}

impl Decision {
    /// Returns a builder seeded with the current UTC timestamp.
    pub fn builder(tenant_id: impl Into<String>, category: DecisionCategory) -> DecisionBuilder {
        DecisionBuilder {
            tenant_id: tenant_id.into(),
            category,
            timestamp: Some(Utc::now()),
            ..Default::default()
        }
    }

    /// Returns a fully populated sample decision (for tests + docs).
    pub fn example() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), serde_json::json!("synapse::decision README"));
        Self {
            decision_id: Uuid::new_v4(),
            tenant_id: "default".into(),
            category: DecisionCategory::ModelSelection,
            scenario: "Choose LLM for production reasoning".into(),
            reasoning: "GPT-4 benchmark advantage justifies 3x cost".into(),
            outcome: DecisionOutcome::Selected("gpt-4".into()),
            confidence: 0.91,
            decision_maker: DecisionMaker::Prefrontal,
            entities: vec!["gpt-4".into(), "gpt-3.5".into()],
            timestamp: Utc::now(),
            valid_from: None,
            valid_until: None,
            metadata,
            caused_by: vec![],
            causes: vec![],
        }
    }
}

/// Fluent builder for `Decision` — lets lobe authors skip boilerplate.
///
/// ```rust
/// use synapse::decision::{Decision, DecisionCategory, DecisionMaker, DecisionOutcome};
/// let d = Decision::builder("tenant1", DecisionCategory::ModelSelection)
///     .scenario("Pick an LLM")
///     .reasoning("GPT-4 is faster")
///     .outcome(DecisionOutcome::Selected("gpt-4".into()))
///     .confidence(0.9)
///     .made_by(DecisionMaker::Prefrontal)
///     .build();
/// # let _ = d;
/// ```
#[derive(Debug, Default, Clone)]
pub struct DecisionBuilder {
    pub(crate) tenant_id: String,
    pub(crate) category: DecisionCategory,
    pub(crate) scenario: String,
    pub(crate) reasoning: String,
    pub(crate) outcome: Option<DecisionOutcome>,
    pub(crate) confidence: f32,
    pub(crate) decision_maker: Option<DecisionMaker>,
    pub(crate) entities: Vec<String>,
    pub(crate) timestamp: Option<DateTime<Utc>>,
    pub(crate) valid_from: Option<DateTime<Utc>>,
    pub(crate) valid_until: Option<DateTime<Utc>>,
    pub(crate) caused_by: Vec<Uuid>,
    pub(crate) metadata: HashMap<String, serde_json::Value>,
}

impl DecisionBuilder {
    pub fn scenario(mut self, s: impl Into<String>) -> Self {
        self.scenario = s.into();
        self
    }

    pub fn reasoning(mut self, s: impl Into<String>) -> Self {
        self.reasoning = s.into();
        self
    }

    pub fn outcome(mut self, o: DecisionOutcome) -> Self {
        self.outcome = Some(o);
        self
    }

    pub fn confidence(mut self, c: f32) -> Self {
        self.confidence = c.clamp(0.0, 1.0);
        self
    }

    pub fn made_by(mut self, m: DecisionMaker) -> Self {
        self.decision_maker = Some(m);
        self
    }

    pub fn entity(mut self, e: impl Into<String>) -> Self {
        self.entities.push(e.into());
        self
    }

    pub fn entities(mut self, e: Vec<String>) -> Self {
        self.entities = e;
        self
    }

    pub fn caused_by(mut self, id: Uuid) -> Self {
        self.caused_by.push(id);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn valid_from(mut self, ts: DateTime<Utc>) -> Self {
        self.valid_from = Some(ts);
        self
    }

    pub fn valid_until(mut self, ts: DateTime<Utc>) -> Self {
        self.valid_until = Some(ts);
        self
    }

    /// Finalize the builder. Panics if `outcome` or `decision_maker`
    /// is missing — these are the two fields a decision cannot exist
    /// without. Use `Decision::builder(...).outcome(...).made_by(...).build()`.
    pub fn build(self) -> Decision {
        let outcome = self.outcome.expect(
            "DecisionBuilder.build: outcome is required (use .outcome(...))",
        );
        let decision_maker = self.decision_maker.expect(
            "DecisionBuilder.build: decision_maker is required (use .made_by(...))",
        );
        let timestamp = self.timestamp.unwrap_or_else(Utc::now);
        Decision {
            decision_id: Uuid::new_v4(),
            tenant_id: self.tenant_id,
            category: self.category,
            scenario: self.scenario,
            reasoning: self.reasoning,
            outcome,
            confidence: self.confidence,
            decision_maker,
            entities: self.entities,
            timestamp,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            metadata: self.metadata,
            caused_by: self.caused_by,
            causes: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_round_trip() {
        let d = Decision::builder("t1", DecisionCategory::ModelSelection)
            .scenario("Pick LLM")
            .reasoning("GPT-4 is faster")
            .outcome(DecisionOutcome::Selected("gpt-4".into()))
            .confidence(0.9)
            .made_by(DecisionMaker::Prefrontal)
            .entity("gpt-4")
            .entity("gpt-3.5")
            .metadata("source", serde_json::json!("test"))
            .build();
        assert_eq!(d.tenant_id, "t1");
        assert_eq!(d.scenario, "Pick LLM");
        assert_eq!(d.reasoning, "GPT-4 is faster");
        assert_eq!(d.entities, vec!["gpt-4", "gpt-3.5"]);
        assert_eq!(d.metadata.get("source").unwrap(), &serde_json::json!("test"));
        assert!((d.confidence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn confidence_is_clamped() {
        let d = Decision::builder("t1", DecisionCategory::Approval)
            .reasoning("x")
            .outcome(DecisionOutcome::Approved)
            .made_by(DecisionMaker::EncephalonOrchestrator)
            .confidence(1.5)
            .build();
        assert!((d.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn category_as_str_round_trip() {
        assert_eq!(DecisionCategory::ModelSelection.as_str(), "model_selection");
        assert_eq!(
            DecisionCategory::Other("custom_thing".into()).as_str(),
            "custom_thing"
        );
    }
}
