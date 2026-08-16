//! Causal-chain traversal on top of `DecisionStore`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::decision::Decision;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CausalDirection {
    /// Walk *up* the dependency tree — "what caused this decision?"
    Upstream,
    /// Walk *down* the tree — "what did this decision cause?"
    Downstream,
}

impl std::fmt::Display for CausalDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Upstream => write!(f, "upstream"),
            Self::Downstream => write!(f, "downstream"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    /// The starting decision id.
    pub root: Uuid,
    pub direction: CausalDirection,
    /// All decisions in traversal order, root at index 0.
    /// Self-references and cycles are detected and skipped.
    pub nodes: Vec<Decision>,
}

impl CausalChain {
    /// Returns the decision at the root of the chain.
    pub fn root_decision(&self) -> Option<&Decision> {
        self.nodes.first()
    }

    /// Returns all decisions other than the root, in traversal order.
    pub fn tail(&self) -> &[Decision] {
        if self.nodes.is_empty() {
            &[]
        } else {
            &self.nodes[1..]
        }
    }

    /// True if any of the chain's decisions violates a confidence
    /// threshold. Useful as a pre-flight check before a downstream
    /// action like "promote this to a WorkflowState.approved_task".
    pub fn has_low_confidence(&self, threshold: f32) -> bool {
        self.nodes.iter().any(|d| d.confidence < threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{Decision, DecisionCategory, DecisionMaker, DecisionOutcome};

    fn mk(id_seed: &str) -> Decision {
        let mut d = Decision::builder(
            "t1",
            DecisionCategory::ModelSelection,
        )
        .scenario(format!("scenario-{}", id_seed))
        .reasoning(format!("reasoning-{}", id_seed))
        .outcome(DecisionOutcome::Selected("gpt-4".into()))
        .made_by(DecisionMaker::Prefrontal)
        .confidence(0.8)
        .build();
        d.decision_id = Uuid::new_v4();
        d
    }

    #[test]
    fn has_low_confidence_works() {
        let mut d = mk("a");
        d.confidence = 0.3;
        let chain = CausalChain {
            root: d.decision_id,
            direction: CausalDirection::Upstream,
            nodes: vec![d],
        };
        assert!(chain.has_low_confidence(0.5));
        assert!(!chain.has_low_confidence(0.2));
    }

    #[test]
    fn tail_excludes_root() {
        let d1 = mk("a");
        let d2 = mk("b");
        let chain = CausalChain {
            root: d1.decision_id,
            direction: CausalDirection::Downstream,
            nodes: vec![d1.clone(), d2.clone()],
        };
        assert_eq!(chain.tail().len(), 1);
        assert_eq!(chain.tail()[0].decision_id, d2.decision_id);
    }
}
