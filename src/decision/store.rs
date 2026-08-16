//! `DecisionStore` trait + `HippocampusDecisionStore` adapter + in-memory store.
//!
//! The trait surface is intentionally small. Every agent lobe (cerebral-cortex,
//! tiangong-agent, paperclip-style orchestrators) talks to `DecisionStore`
//! via the same API; the storage choice is a deployment concern.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::causal::{CausalChain, CausalDirection};
use super::decision::{Decision, DecisionCategory, DecisionMaker, DecisionOutcome};

/// Query parameters for `DecisionStore::query`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DecisionQuery {
    pub tenant_id: String,
    pub category: Option<DecisionCategory>,
    pub decision_maker: Option<DecisionMaker>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub min_confidence: Option<f32>,
    pub limit: Option<usize>,
    pub caused_by: Option<Uuid>,
    pub causes: Option<Uuid>,
}

/// `DecisionStore` — lobe-agnostic persistence for first-class decisions.
///
/// Implementations are responsible for:
/// - Persisting `Decision` records (graph, RDBMS, KV, file, …)
/// - Maintaining causal links (`caused_by` / `causes`) between decisions
/// - Surfacing `Decision` for retrieval and causal-chain traversal
///
/// Hippocampus is the canonical backend (`HippocampusDecisionStore`)
/// but the trait allows custom stores (e.g. Redis with RediSearch,
/// S3 + Athena, Postgres + pgvector) for production deployments.
#[async_trait]
pub trait DecisionStore: Send + Sync {
    /// Persist a new decision. Returns the stored decision (the
    /// `decision_id` field is guaranteed set; storage backends may
    /// add their own bookkeeping via `metadata`).
    async fn record(&self, decision: Decision) -> DecisionStoreResult<Decision>;

    /// Fetch a single decision by ID.
    async fn get(&self, decision_id: Uuid) -> DecisionStoreResult<Option<Decision>>;

    /// Update an existing decision. Returns the post-update decision.
    async fn update(&self, decision: Decision) -> DecisionStoreResult<Decision>;

    /// Soft-delete: mark `valid_until = now` without removing the record.
    /// Hard-delete is intentionally not supported — decisions are an
    /// audit signal and must be retained for compliance.
    async fn supersede(&self, decision_id: Uuid, at: DateTime<Utc>) -> DecisionStoreResult<()>;

    /// Query decisions by filter.
    async fn query(&self, q: DecisionQuery) -> DecisionStoreResult<Vec<Decision>>;

    /// Trace causal chain in `direction` (`Upstream` or `Downstream`)
    /// up to `max_depth` hops. Returns the chain as an ordered list of
    /// `Decision` nodes (root at index 0).
    async fn causal_chain(
        &self,
        decision_id: Uuid,
        direction: CausalDirection,
        max_depth: usize,
    ) -> DecisionStoreResult<CausalChain>;
}

pub type DecisionStoreResult<T> = Result<T, DecisionStoreError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecisionStoreError {
    #[error("not found: {0}")]
    NotFound(Uuid),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
}

#[async_trait]
impl<T: DecisionStore + ?Sized> DecisionStore for Arc<T> {
    async fn record(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        (**self).record(decision).await
    }
    async fn get(&self, decision_id: Uuid) -> DecisionStoreResult<Option<Decision>> {
        (**self).get(decision_id).await
    }
    async fn update(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        (**self).update(decision).await
    }
    async fn supersede(&self, decision_id: Uuid, at: DateTime<Utc>) -> DecisionStoreResult<()> {
        (**self).supersede(decision_id, at).await
    }
    async fn query(&self, q: DecisionQuery) -> DecisionStoreResult<Vec<Decision>> {
        (**self).query(q).await
    }
    async fn causal_chain(
        &self,
        decision_id: Uuid,
        direction: CausalDirection,
        max_depth: usize,
    ) -> DecisionStoreResult<CausalChain> {
        (**self).causal_chain(decision_id, direction, max_depth).await
    }
}

/// In-memory `DecisionStore` for tests + docs + small deployments.
#[derive(Debug, Default)]
pub struct InMemoryDecisionStore {
    inner: RwLock<InMemoryInner>,
}

#[derive(Debug, Default)]
struct InMemoryInner {
    decisions: HashMap<Uuid, Decision>,
}

impl InMemoryDecisionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DecisionStore for InMemoryDecisionStore {
    async fn record(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        let mut g = self.inner.write().await;
        g.decisions.insert(decision.decision_id, decision.clone());
        Ok(decision)
    }

    async fn get(&self, decision_id: Uuid) -> DecisionStoreResult<Option<Decision>> {
        Ok(self.inner.read().await.decisions.get(&decision_id).cloned())
    }

    async fn update(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        let mut g = self.inner.write().await;
        g.decisions.insert(decision.decision_id, decision.clone());
        Ok(decision)
    }

    async fn supersede(&self, decision_id: Uuid, at: DateTime<Utc>) -> DecisionStoreResult<()> {
        let mut g = self.inner.write().await;
        let entry = g.decisions.get_mut(&decision_id).ok_or(DecisionStoreError::NotFound(decision_id))?;
        entry.valid_until = Some(at);
        Ok(())
    }

    async fn query(&self, q: DecisionQuery) -> DecisionStoreResult<Vec<Decision>> {
        let g = self.inner.read().await;
        let mut out: Vec<Decision> = g
            .decisions
            .values()
            .filter(|d| d.tenant_id == q.tenant_id)
            .filter(|d| q.category.as_ref().map_or(true, |c| &d.category == c))
            .filter(|d| q.decision_maker.as_ref().map_or(true, |m| &d.decision_maker == m))
            .filter(|d| q.since.map_or(true, |s| d.timestamp >= s))
            .filter(|d| q.until.map_or(true, |u| d.timestamp <= u))
            .filter(|d| q.min_confidence.map_or(true, |c| d.confidence >= c))
            .filter(|d| q.caused_by.map_or(true, |p| d.caused_by.contains(&p)))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if let Some(limit) = q.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    async fn causal_chain(
        &self,
        decision_id: Uuid,
        direction: CausalDirection,
        max_depth: usize,
    ) -> DecisionStoreResult<CausalChain> {
        let g = self.inner.read().await;
        let mut chain = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut frontier = vec![(decision_id, 0usize)];

        while let Some((current, depth)) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }
            let Some(node) = g.decisions.get(&current) else { continue };

            let ids = match direction {
                CausalDirection::Upstream => &node.caused_by,
                CausalDirection::Downstream => &node.causes,
            };
            chain.push(node.clone());
            if depth < max_depth {
                for next in ids {
                    if !visited.contains(next) {
                        frontier.push((*next, depth + 1));
                    }
                }
            }
        }
        Ok(CausalChain {
            root: decision_id,
            direction,
            nodes: chain,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HippocampusConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl HippocampusConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("HIPPOCAMPUS_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:60080".into()),
            api_key: std::env::var("HIPPOCAMPUS_API_KEY").ok(),
        }
    }
}

/// `DecisionStore` impl that talks to a running `hippocampus` server
/// over HTTP. The transport is intentionally `reqwest`-based so this
/// works inside WASM, A2A handlers, and standalone `lobe_app` binaries
/// without an extra runtime.
pub struct HippocampusDecisionStore {
    cfg: HippocampusConfig,
    http: reqwest::Client,
}

impl HippocampusDecisionStore {
    pub fn new(cfg: HippocampusConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("reqwest build");
        Self { cfg, http }
    }
}

#[async_trait]
impl DecisionStore for HippocampusDecisionStore {
    async fn record(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        let url = format!("{}/api/decisions", self.cfg.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .json(&decision)
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        let stored: Decision = resp
            .json()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        Ok(stored)
    }

    async fn get(&self, decision_id: Uuid) -> DecisionStoreResult<Option<Decision>> {
        let url = format!("{}/api/decisions/{}", self.cfg.base_url, decision_id);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        let d: Decision = resp
            .json()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        Ok(Some(d))
    }

    async fn update(&self, decision: Decision) -> DecisionStoreResult<Decision> {
        let url = format!("{}/api/decisions/{}", self.cfg.base_url, decision.decision_id);
        let resp = self
            .http
            .put(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .json(&decision)
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        let d: Decision = resp
            .json()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        Ok(d)
    }

    async fn supersede(&self, decision_id: Uuid, at: DateTime<Utc>) -> DecisionStoreResult<()> {
        let url = format!(
            "{}/api/decisions/{}/invalidate?at={}",
            self.cfg.base_url,
            decision_id,
            at.to_rfc3339()
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        Ok(())
    }

    async fn query(&self, q: DecisionQuery) -> DecisionStoreResult<Vec<Decision>> {
        let url = format!("{}/api/decisions", self.cfg.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .json(&q)
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        let v: Vec<Decision> = resp
            .json()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        Ok(v)
    }

    async fn causal_chain(
        &self,
        decision_id: Uuid,
        direction: CausalDirection,
        max_depth: usize,
    ) -> DecisionStoreResult<CausalChain> {
        let url = format!(
            "{}/api/decisions/{}/causal-chain?direction={}&max_depth={}",
            self.cfg.base_url,
            decision_id,
            format!("{:?}", direction).to_lowercase(),
            max_depth
        );
        let resp = self
            .http
            .get(&url)
            .bearer_auth(self.cfg.api_key.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DecisionStoreError::Backend(format!("HTTP {}", resp.status())));
        }
        let chain: CausalChain = resp
            .json()
            .await
            .map_err(|e| DecisionStoreError::Backend(e.to_string()))?;
        Ok(chain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::Decision;

    fn mk(tenant: &str, outcome: DecisionOutcome) -> Decision {
        Decision::builder(tenant, DecisionCategory::Approval)
            .scenario("x")
            .reasoning("y")
            .outcome(outcome)
            .made_by(DecisionMaker::CortexOrchestrator)
            .confidence(1.0)
            .build()
    }

    #[tokio::test]
    async fn in_memory_round_trip() {
        let store = InMemoryDecisionStore::new();
        let d = mk("t1", DecisionOutcome::Approved);
        let id = d.decision_id;
        store.record(d.clone()).await.unwrap();
        let fetched = store.get(id).await.unwrap().unwrap();
        assert_eq!(fetched.scenario, "x");
    }

    #[tokio::test]
    async fn in_memory_supersede_sets_valid_until() {
        let store = InMemoryDecisionStore::new();
        let d = mk("t1", DecisionOutcome::Approved);
        let id = d.decision_id;
        store.record(d).await.unwrap();
        let at = Utc::now();
        store.supersede(id, at).await.unwrap();
        let fetched = store.get(id).await.unwrap().unwrap();
        assert_eq!(fetched.valid_until, Some(at));
    }

    #[tokio::test]
    async fn in_memory_query_filters_by_tenant() {
        let store = InMemoryDecisionStore::new();
        store.record(mk("t1", DecisionOutcome::Approved)).await.unwrap();
        store.record(mk("t2", DecisionOutcome::Rejected)).await.unwrap();
        let q = DecisionQuery {
            tenant_id: "t1".into(),
            ..Default::default()
        };
        let res = store.query(q).await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].tenant_id, "t1");
    }
}
