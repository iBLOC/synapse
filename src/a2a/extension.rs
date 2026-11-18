//! A2A v1.0 extension activation.
//!
//! The A2A v1.0 spec lets an agent declare extra capabilities in its
//! AgentCard (AgentCapabilities.extensions) and lets a client opt into
//! them per request with a header carrying the extension URIs.
//!
//! ## Why this module exists
//!
//! Before this module, Synapse hard-coded capabilities.extensions: None
//! and ignored the request header entirely. That is not "no extensions"
//! -- it is "the negotiation never happens": a client that asks for an
//! extension gets no acknowledgement, so it cannot tell whether the agent
//! understood the request. The failure is silent, and silent failures
//! look exactly like success.
//!
//! ## Header naming -- two names, one meaning
//!
//! The A2A v1.0 specification names the header A2A-Extensions. Earlier
//! drafts (and Google's ADK, which shipped its A2A extension against
//! those drafts) used X-A2A-Extensions. A server that only accepts the
//! current name rejects every ADK client written against the older one,
//! and a server that only accepts the old name rejects every spec-current
//! client. So **both are accepted**, and both are answered.
//!
//! ## What activation does and does not do
//!
//! Activation is **negotiation, not implementation**. When a client asks
//! for an extension Synapse does not implement, the honest answer is to
//! not activate it -- silently pretending would be exactly the silent
//! failure described above. Synapse therefore:
//!
//! * echoes back the extensions it actually activated, so the client can
//!   see which ones were honoured;
//! * reports the ones it could not honour, so the gap is visible rather
//!   than assumed away.

use std::collections::HashSet;

/// The header a spec-current client sends to request extensions.
///
/// Per the A2A v1.0 specification, section 14.2.2 "A2A-Extensions Header".
pub const HEADER_A2A_EXTENSIONS: &str = "a2a-extensions";

/// The header earlier drafts (and Google's ADK) use for the same purpose.
///
/// ADK documents X-A2A-Extensions as its activation mechanism; the
/// A2A v1.0 spec settled on A2A-Extensions. Both are read, because
/// the difference is a naming artifact, not a behavioural one.
pub const HEADER_X_A2A_EXTENSIONS: &str = "x-a2a-extensions";

/// The metadata key under which a server reports the extensions it
/// activated for a request.
///
/// ADK echoes the activated list in the response metadata; A2A's own
/// samples use the same shape. Kept as a constant so the server, the
/// client and the tests all agree on one spelling.
pub const METADATA_KEY_ACTIVATED_EXTENSIONS: &str = "activatedExtensions";

/// Parse a comma-separated extension header value into a list of URIs.
///
/// The spec says multiple values for the same service parameter SHOULD be
/// comma-separated in a single header field, and that several header lines
/// may also appear. Both shapes are handled: this function is applied to
/// each header value, and the results are unioned by the caller.
///
/// Whitespace around each URI is trimmed and empty entries are dropped,
/// so "a, ,b " yields exactly ["a", "b"].
pub fn parse_extension_header(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Read the extension URIs requested by a client out of its request
/// headers.
///
/// Header names are compared case-insensitively (the spec's header names
/// are case-insensitive by HTTP definition, and the SDK lowercases them
/// before they reach us -- but a caller may hand us raw names, so the
/// comparison is done here rather than assumed).
///
/// Returns the union of both accepted header spellings, in the order they
/// first appear, with duplicates removed.
pub fn requested_extensions(headers: &std::collections::HashMap<String, Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for name in [HEADER_A2A_EXTENSIONS, HEADER_X_A2A_EXTENSIONS] {
        let Some(values) = headers.get(name) else {
            continue;
        };
        for value in values {
            for uri in parse_extension_header(value) {
                if seen.insert(uri.clone()) {
                    out.push(uri);
                }
            }
        }
    }
    out
}

/// The outcome of negotiating an extension request against what the agent
/// actually supports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtensionNegotiation {
    /// Extensions the client asked for **and** the agent supports.
    pub activated: Vec<String>,
    /// Extensions the client asked for that the agent does not support.
    ///
    /// Reported rather than swallowed: an agent that quietly drops an
    /// unsupported extension makes the client's request look honoured.
    pub unsupported: Vec<String>,
}

impl ExtensionNegotiation {
    /// Negotiate a client's request against the agent's supported set.
    ///
    /// supported is the agent's declared extension set (the URIs it
    /// published in its AgentCard). An empty supported set means the
    /// agent implements no extensions, so every requested URI lands in
    /// unsupported.
    pub fn negotiate(requested: &[String], supported: &[String]) -> ExtensionNegotiation {
        let supported_set: HashSet<&str> = supported.iter().map(|s| s.as_str()).collect();
        let mut activated = Vec::new();
        let mut unsupported = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for uri in requested {
            if !seen.insert(uri.as_str()) {
                continue;
            }
            if supported_set.contains(uri.as_str()) {
                activated.push(uri.clone());
            } else {
                unsupported.push(uri.clone());
            }
        }
        ExtensionNegotiation {
            activated,
            unsupported,
        }
    }

    /// Whether any extension was activated for this request.
    pub fn is_empty(&self) -> bool {
        self.activated.is_empty()
    }
}

/// Attach the negotiation outcome to an A2A stream response.
///
/// The activated list goes under [`METADATA_KEY_ACTIVATED_EXTENSIONS`];
/// unsupported ones under `unsupportedExtensions`. Both are written into
/// the event metadata so a client can see exactly what the agent honoured
/// -- a client that asked for an extension and got silence has no way to
/// tell "not supported" from "ignored", and those need different
/// reactions.
///
/// An event that already carries metadata (the tool-event side channel,
/// for instance) keeps it: the extension keys are merged in, not
/// substituted.
pub fn apply_negotiation_metadata(
    response: a2a::StreamResponse,
    negotiation: &ExtensionNegotiation,
) -> a2a::StreamResponse {
    if negotiation.activated.is_empty() && negotiation.unsupported.is_empty() {
        return response;
    }
    let mut meta: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    if !negotiation.activated.is_empty() {
        meta.insert(
            METADATA_KEY_ACTIVATED_EXTENSIONS.to_string(),
            serde_json::Value::Array(
                negotiation
                    .activated
                    .iter()
                    .map(|u| serde_json::Value::String(u.clone()))
                    .collect(),
            ),
        );
    }
    if !negotiation.unsupported.is_empty() {
        meta.insert(
            "unsupportedExtensions".to_string(),
            serde_json::Value::Array(
                negotiation
                    .unsupported
                    .iter()
                    .map(|u| serde_json::Value::String(u.clone()))
                    .collect(),
            ),
        );
    }
    match response {
        a2a::StreamResponse::StatusUpdate(mut ev) => {
            let m = ev.metadata.get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in meta {
                m.insert(k, v);
            }
            a2a::StreamResponse::StatusUpdate(ev)
        }
        a2a::StreamResponse::ArtifactUpdate(mut ev) => {
            let m = ev.metadata.get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in meta {
                m.insert(k, v);
            }
            a2a::StreamResponse::ArtifactUpdate(ev)
        }
        other => other,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_splits_comma_and_trims() {
        assert_eq!(
            parse_extension_header("https://a/v1, https://b/v1"),
            vec!["https://a/v1", "https://b/v1"]
        );
    }

    #[test]
    fn parse_drops_empty_entries() {
        assert_eq!(
            parse_extension_header("https://a/v1, ,  ,https://b/v1"),
            vec!["https://a/v1", "https://b/v1"]
        );
        assert!(parse_extension_header("").is_empty());
        assert!(parse_extension_header("   ").is_empty());
    }

    #[test]
    fn reads_the_current_spec_header() {
        let mut h = HashMap::new();
        h.insert(
            HEADER_A2A_EXTENSIONS.to_string(),
            vec!["https://a/v1".to_string()],
        );
        assert_eq!(requested_extensions(&h), vec!["https://a/v1"]);
    }

    #[test]
    fn reads_the_legacy_adk_header() {
        // ADK still sends X-A2A-Extensions; a server that ignores it
        // silently drops the client's intent.
        let mut h = HashMap::new();
        h.insert(
            HEADER_X_A2A_EXTENSIONS.to_string(),
            vec!["https://adk/v1".to_string()],
        );
        assert_eq!(requested_extensions(&h), vec!["https://adk/v1"]);
    }

    #[test]
    fn reads_both_spellings_and_dedupes() {
        let mut h = HashMap::new();
        h.insert(
            HEADER_A2A_EXTENSIONS.to_string(),
            vec!["https://a/v1".to_string()],
        );
        h.insert(
            HEADER_X_A2A_EXTENSIONS.to_string(),
            vec!["https://a/v1,https://b/v1".to_string()],
        );
        assert_eq!(
            requested_extensions(&h),
            vec!["https://a/v1", "https://b/v1"]
        );
    }

    #[test]
    fn negotiation_activates_only_supported() {
        let req = vec!["https://a/v1".to_string(), "https://z/v1".to_string()];
        let sup = vec!["https://a/v1".to_string()];
        let n = ExtensionNegotiation::negotiate(&req, &sup);
        assert_eq!(n.activated, vec!["https://a/v1"]);
        assert_eq!(n.unsupported, vec!["https://z/v1"]);
        assert!(!n.is_empty());
    }

    #[test]
    fn negotiation_reports_everything_unsupported_when_agent_has_none() {
        let req = vec!["https://a/v1".to_string()];
        let n = ExtensionNegotiation::negotiate(&req, &[]);
        assert!(n.activated.is_empty());
        assert_eq!(n.unsupported, vec!["https://a/v1"]);
        assert!(n.is_empty());
    }

    #[test]
    fn negotiation_dedupes_repeated_requests() {
        let req = vec!["https://a/v1".to_string(), "https://a/v1".to_string()];
        let sup = vec!["https://a/v1".to_string()];
        let n = ExtensionNegotiation::negotiate(&req, &sup);
        assert_eq!(n.activated, vec!["https://a/v1"]);
    }
}