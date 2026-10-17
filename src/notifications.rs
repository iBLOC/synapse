//! Cross-crate notification types.
//!
//! 2026-10-17: a shared `RuleEvent` so producers (diencephalon
//! thalamus) and consumers (temporal lobe + downstream
//! notification sinks) agree on a wire format. The producer still
//! keeps its own concrete struct for in-process broadcast
//! (`diencephalon::zenoh_source::RuleEvent`); this module mirrors
//! the fields so consumers can depend only on synapse, not on
//! diencephalon's internals. Conversion is `from`/`try_into`.
//!
//! In multi-process deployments, thalamus serializes a
//! `RuleEvent` over the wire (zenoh pub / gRPC / etc.) and the
//! receiver materializes it back into a local struct via
//! `RuleEvent::from_wire` (which is just a `From<synapse version>`
//! since the fields are identical).

use serde::{Deserialize, Serialize};

/// One rule-fire notification. Field names match
/// `diencephalon::zenoh_source::RuleEvent` (see that module's
/// doc for the rule-engine hot path).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleEvent {
    pub rule_id: String,
    pub sensor: String,
    pub value: f64,
    pub matched_rule: Option<usize>,
    pub ts_ms: i64,
}

impl RuleEvent {
    /// Render a short human-readable description for chat / log /
    /// push surfaces. Kept here (not in the producer) so the
    /// format is owned by the consumer; producers stay
    /// presentation-agnostic.
    pub fn render(&self) -> String {
        let matched = self
            .matched_rule
            .map(|i| format!(" (rule #{i})"))
            .unwrap_or_default();
        format!(
            "[{}] rule `{}` fired: sensor={} value={}{}",
            self.ts_ms, self.rule_id, self.sensor, self.value, matched
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_all_fields() {
        let e = RuleEvent {
            rule_id: "sensor.bath_temp".into(),
            sensor: "bath_temp".into(),
            value: 27.3,
            matched_rule: Some(0),
            ts_ms: 1_700_000_000_000,
        };
        let s = e.render();
        assert!(s.contains("sensor.bath_temp"), "got: {s}");
        assert!(s.contains("bath_temp"), "got: {s}");
        assert!(s.contains("27.3"), "got: {s}");
        assert!(s.contains("rule #0"), "got: {s}");
        assert!(s.contains("1700000000000"), "got: {s}");
    }

    #[test]
    fn render_omits_matched_rule_when_none() {
        let e = RuleEvent {
            rule_id: "r".into(),
            sensor: "s".into(),
            value: 1.0,
            matched_rule: None,
            ts_ms: 0,
        };
        let s = e.render();
        assert!(!s.contains("rule #"), "got: {s}");
        assert!(s.contains("r") && s.contains("s") && s.contains("1"), "got: {s}");
    }

    #[test]
    fn round_trip_serde() {
        let e = RuleEvent {
            rule_id: "r1".into(),
            sensor: "s1".into(),
            value: -1.5,
            matched_rule: Some(2),
            ts_ms: 99,
        };
        let j = serde_json::to_string(&e).unwrap();
        let back: RuleEvent = serde_json::from_str(&j).unwrap();
        assert_eq!(e, back);
    }
}
