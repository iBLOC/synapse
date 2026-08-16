//! Helper to register an agent with a service registry on startup.
//!
//! POSTs to `{registry_url}/api/agents/register` with the agent's
//! AgentCard. Best-effort: on failure, log a warning. The agent should
//! still work standalone when REGISTRY_URL is unset.

use std::time::Duration;

use crate::lobe::AgentCard;

/// Register an agent with a service registry.
///
/// The registry URL should be provided without trailing slash.
/// The function will POST to `{registry_url}/api/agents/register`.
///
/// # Arguments
/// * `registry_url` - The base URL of the service registry
/// * `card` - The agent's AgentCard to register
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` on failure (network error, non-2xx response)
pub async fn register_with_registry(
    registry_url: &str,
    card: &AgentCard,
) -> Result<(), String> {
    let url = format!("{}/api/agents/register", registry_url.trim_end_matches('/'));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_with_registry_empty_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = AgentCard::new("test", "test agent", "http://localhost:8080", "1.0.0");
        let result = rt.block_on(register_with_registry("", &card));
        assert!(result.is_err());
    }

    #[test]
    fn register_with_registry_invalid_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = AgentCard::new("test", "test agent", "http://localhost:8080", "1.0.0");
        let result = rt.block_on(register_with_registry("not-a-url", &card));
        assert!(result.is_err());
    }
}
