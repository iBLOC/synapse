//! URL locality classifier: does this base_url leave the device?
//!
//! Providers pointed at loopback / private ranges / mDNS names run
//! model inference on-device (neuron embedded, LAN gateways) — their
//! traffic is not an external egress and must NOT be redacted.

/// `true` when `url` points outside the local trust boundary
/// (loopback, RFC1918, link-local, `*.local` / `*.localhost`).
/// Unparseable input classifies as external — the privacy-fail-closed
/// direction for a redaction gate.
pub fn is_external_url(url: &str) -> bool {
    match parse_host(url) {
        Some(host) => !is_local_host(&host),
        None => true,
    }
}

fn parse_host(input: &str) -> Option<String> {
    let rest = input.split_once("://").map(|(_, r)| r).unwrap_or(input);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
    let host = if let Some(after_bracket) = authority.strip_prefix('[') {
        // IPv6 literal: [::1]:8085 -> ::1
        after_bracket
            .split_once(']')
            .map(|(h, _)| h)
            .unwrap_or(after_bracket)
    } else if authority.matches(':').count() > 1 {
        // Bare IPv6 with more than one colon — per RFC a port would
        // require brackets, so the whole authority is the address.
        authority
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

fn is_local_host(host: &str) -> bool {
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    if let Ok(v4) = host.parse::<std::net::Ipv4Addr>() {
        return v4.is_loopback() || v4.is_private() || v4.is_link_local();
    }
    if let Ok(v6) = host.parse::<std::net::Ipv6Addr>() {
        return v6.is_loopback() || (v6.segments()[0] & 0xffc0) == 0xfe80;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_endpoints_are_external() {
        assert!(is_external_url("https://api.deepseek.com/v1"));
        assert!(is_external_url("https://dashscope.aliyuncs.com/compatible-mode/v1"));
        assert!(is_external_url("https://api.openai.com"));
        assert!(is_external_url("http://203.0.113.7:9000/v1"));
    }

    #[test]
    fn loopback_and_private_are_local() {
        assert!(!is_external_url("http://localhost:8085/v1"));
        assert!(!is_external_url("http://127.0.0.1:8085/v1"));
        assert!(!is_external_url("http://[::1]:9000/v1"));
        assert!(!is_external_url("http://192.168.1.5:8083"));
        assert!(!is_external_url("http://10.0.0.12:8082"));
        assert!(!is_external_url("http://172.16.4.4"));
        assert!(!is_external_url("http://169.254.7.7:80"));
        assert!(!is_external_url("http://fe80::1:9000"));
    }

    #[test]
    fn local_names_and_userinfo() {
        assert!(!is_external_url("http://neuron.local:8085"));
        assert!(!is_external_url("http://sub.localhost:8085"));
        // userinfo must be stripped before host classification
        assert!(!is_external_url("http://user:pass@127.0.0.1:8085"));
        assert!(is_external_url("https://user:pass@api.example.com/v1"));
    }

    #[test]
    fn garbage_fails_closed_to_external() {
        assert!(is_external_url(""));
        assert!(is_external_url("   "));
        assert!(is_external_url("::::"));
    }
}
