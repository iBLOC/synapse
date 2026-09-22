//! Helper to register a native lobe with Cortex (L3.5).
//!
//! POSTs to `{encephalon_url}/encephalon/lobes/{name}` with the lobe's
//! AgentCard. Best-effort: on failure, log a warning. The lobe
//! should still work standalone when ENCEPHALON_URL is unset.
//!
//! This complements `register_with_registry` (which targets a
//! generic agent registry). Use this when you specifically want
//! the encephalon lobe cache to know about you.
//!
//! # Example
//! ```ignore
//! if let Ok(encephalon_url) = std::env::var("ENCEPHALON_URL") {
//!     let card = AgentCard::new("temporal", "...",
//!         &format!("http://{bind}"), env!("CARGO_PKG_VERSION"));
//!     register_with_encephalon(&encephalon_url, &card).await?;
//! }
//! ```

use std::time::Duration;

use crate::lobe::AgentCard;

pub async fn register_with_encephalon(
    encephalon_url: &str,
    card: &AgentCard,
) -> Result<(), String> {
    let base = encephalon_url.trim_end_matches('/');
    let url = format!("{}/encephalon/lobes/{}", base, card.name);
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
        return Err(format!("register_with_encephalon failed: {} {}", status, txt));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_with_encephalon_empty_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = AgentCard::new("test", "test agent", "http://localhost:8090", "1.0.0");
        let result = rt.block_on(register_with_encephalon("", &card));
        assert!(result.is_err());
    }

    #[test]
    fn register_with_encephalon_invalid_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = AgentCard::new("test", "test agent", "http://localhost:8090", "1.0.0");
        let result = rt.block_on(register_with_encephalon("not-a-url", &card));
        assert!(result.is_err());
    }
}
