//! The HTTP client for endpoints that may live on this machine.
//!
//! ⚠⚠ **回环端点不许走系统代理** ——
//!
//! reqwest picks the proxy up from the operating system (on Windows:
//! the registry's `ProxyServer`) and the system's own bypass list --
//! `ProxyOverride`, which is exactly where `localhost` and `127.*`
//! live -- **does not take effect for loopback**. So on a machine with
//! a system proxy, an endpoint on `127.0.0.1` is handed to the proxy,
//! which cannot reach the machine's own loopback interface and answers
//! `502 Bad Gateway` with an empty body.
//!
//! Measured on the machine this was found on: a plain
//! `reqwest::Client::new()` against a loopback peer got `502` with an
//! empty body **and the peer was never reached**; the same client with
//! `.no_proxy()` got `200`. A remote endpoint keeps the system proxy
//! -- it may well need it.
//!
//! ⚠⚠ And do **not** lean on the system's list: hyper-util reads the
//! registry's `ProxyOverride` only when **no** `NO_PROXY`/`no_proxy` is
//! set in the environment -- one environment variable silently
//! replaces the whole registry list. "The system knows to bypass
//! loopback" is not something a product can rely on.

/// Whether the endpoint lives on this machine's loopback interface.
///
/// `localhost`, `127.0.0.0/8` and `::1` qualify. Private LAN ranges
/// deliberately do not: some deployments really do route those
/// through a proxy.
pub fn is_loopback(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_matches(['[', ']']);
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => ip.octets()[0] == 127,
        Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => host.eq_ignore_ascii_case("localhost"),
    }
}

/// A client builder that bypasses the proxy for a loopback endpoint.
///
/// Use it wherever an endpoint may be on this machine: the A2A peer,
/// the end-brain registration, the decision store, the SSE transport.
pub fn builder_for(url: &str) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder();
    if is_loopback(url) {
        builder.no_proxy()
    } else {
        builder
    }
}

/// A client that bypasses the proxy for a loopback endpoint.
pub fn client_for(url: &str) -> reqwest::Client {
    builder_for(url).build().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loopback_endpoint_is_recognised() {
        for url in [
            "http://127.0.0.1:9099",
            "http://127.0.0.1/",
            "https://127.1.2.3/a2a",
            "http://localhost:8001/a2a/agent",
            "http://[::1]:9099",
        ] {
            assert!(is_loopback(url), "{url} is loopback");
        }

        for url in [
            "http://example.com",
            "https://peer.example.com:443/a2a",
            "http://192.168.1.10:9099",
            "http://10.0.0.5",
            "http://127.example.com",
            "not a url",
        ] {
            assert!(!is_loopback(url), "{url} is not loopback");
        }
    }
}
