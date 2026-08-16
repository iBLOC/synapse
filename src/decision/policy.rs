//! `PolicyEngine` — versioned compliance rules that gate every decision.
//!
//! Mirrors Semantica's `PolicyEngine.check_compliance()`: a decision is
//! only recorded *after* it passes the active policy set. This makes
//! policy violations first-class audit signals rather than post-hoc
//! log analysis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Semantic version of a policy. String so that "1.2", "2.0-rc.1", and
/// "internal-2024Q4" all work without ceremony.
pub type PolicyVersion = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub policy_id: Uuid,
    pub name: String,
    pub description: String,
    pub rules: HashMap<String, serde_json::Value>,
    pub category: String,
    pub version: PolicyVersion,
    pub effective_date: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Policy {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<PolicyVersion>,
        category: impl Into<String>,
        rules: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            policy_id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            rules,
            category: category.into(),
            version: version.into(),
            effective_date: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn with_metadata(mut self, k: impl Into<String>, v: serde_json::Value) -> Self {
        self.metadata.insert(k.into(), v);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceResult {
    Compliant,
    Violation { policy_id: Uuid, reason: String },
    MissingPolicy { policy_name: String },
}

impl ComplianceResult {
    pub fn is_compliant(&self) -> bool {
        matches!(self, Self::Compliant)
    }
}

#[derive(Debug, Default, Clone)]
pub struct PolicyEngine {
    policies: HashMap<String, Policy>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, policy: Policy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    pub fn remove(&mut self, name: &str) -> Option<Policy> {
        self.policies.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Policy> {
        self.policies.get(name)
    }

    pub fn list(&self) -> Vec<&Policy> {
        self.policies.values().collect()
    }

    /// Run the active policy set against a decision payload.
    /// `decision_payload` is the structured input to the policy rules
    /// (typically a flattened `Decision` + extra context).
    /// If a named policy is missing, returns `MissingPolicy` (caller
    /// decides whether that's a hard fail or a warning).
    pub fn check_compliance(
        &self,
        decision_payload: &serde_json::Value,
        policy_names: &[String],
    ) -> Vec<ComplianceResult> {
        let mut results = Vec::new();
        for name in policy_names {
            let policy = match self.policies.get(name) {
                Some(p) => p,
                None => {
                    results.push(ComplianceResult::MissingPolicy { policy_name: name.clone() });
                    continue;
                }
            };
            if let Some(reason) = evaluate_rules(&policy.rules, decision_payload) {
                results.push(ComplianceResult::Violation {
                    policy_id: policy.policy_id,
                    reason,
                });
            } else {
                results.push(ComplianceResult::Compliant);
            }
        }
        results
    }
}

fn evaluate_rules(
    rules: &HashMap<String, serde_json::Value>,
    payload: &serde_json::Value,
) -> Option<String> {
    for (key, expected) in rules {
        let actual = payload.get(key);
        if !matches_requirement(actual, expected) {
            return Some(format!(
                "rule '{}' violated: expected {:?}, got {:?}",
                key, expected, actual
            ));
        }
    }
    None
}

fn matches_requirement(actual: Option<&serde_json::Value>, expected: &serde_json::Value) -> bool {
    match (actual, expected) {
        (Some(a), e) if a == e => true,
        // special-case: expected `true` requires key to be present and truthy.
        (Some(a), e) if e == &serde_json::Value::Bool(true) => a.as_bool().unwrap_or(false),
        // special-case: expected `false` permits missing key.
        (_, e) if e == &serde_json::Value::Bool(false) => true,
        (None, _) => false,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn engine_with_pii_policy() -> PolicyEngine {
        let mut e = PolicyEngine::new();
        let mut rules = HashMap::new();
        rules.insert("requires_consent".into(), json!(true));
        rules.insert("user_consent".into(), json!(true));
        e.add(
            Policy::new("data_privacy", "1.2", "privacy", rules)
                .with_description("No PII without consent"),
        );
        e
    }

    #[test]
    fn compliant_decision() {
        let e = engine_with_pii_policy();
        let payload = json!({
            "requires_consent": true,
            "user_consent": true,
        });
        let results = e.check_compliance(&payload, &["data_privacy".into()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_compliant());
    }

    #[test]
    fn violation_detected() {
        let e = engine_with_pii_policy();
        let payload = json!({
            "requires_consent": true,
            "user_consent": false,
        });
        let results = e.check_compliance(&payload, &["data_privacy".into()]);
        assert_eq!(results.len(), 1);
        match &results[0] {
            ComplianceResult::Violation { reason, .. } => {
                assert!(reason.contains("user_consent"));
            }
            _ => panic!("expected Violation"),
        }
    }

    #[test]
    fn missing_policy_reported() {
        let e = PolicyEngine::new();
        let payload = json!({});
        let results = e.check_compliance(&payload, &["nonexistent".into()]);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], ComplianceResult::MissingPolicy { .. }));
    }
}
