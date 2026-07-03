//! Helper to register a lobe with cortex on startup.
//!
//! POSTs to `{cortex_url}/api/cortex/agents/register` with the agent's
//! AgentCard. Best-effort: on failure, log a warning. The lobe should
//! still work standalone when CORTEX_URL is unset.

use std::time::Duration;

use crate::lobe::AgentCard;

pub async fn register_with_cortex(
    cortex_url: &str,
    card: &AgentCard,
) -> Result<(), String> {
    let url = format!("{}/api/cortex/agents/register", cortex_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("reqwest: {e}"))?;
    let body = serde_json::json!({
        "kind": "native",
        "url": card.url,
        "agent_card": card,
    });
    let resp = client.post(&url).json(&body).send().await
        .map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(format!("register failed: {status} {txt}"));
    }
    Ok(())
}
