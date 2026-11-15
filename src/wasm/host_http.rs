//! Controlled outbound HTTP for wasm modules (host capability, ABI v1).
//!
//! This closes the one gap that stopped "an open-source HTTP-wrapper MCP
//! packaged as wasm" from running in-process: the old `WasmRuntime` was a
//! pure string-in/string-out sandbox with **no linker imports at all**, so a
//! module could not reach the network. Here we add exactly one import —
//! `env.http_request` — and put a security gate in front of it.
//!
//! Ownership split (deliberate): **synapse owns the guard, the ABI, the
//! limits and the buffer discipline**; the **caller injects the transport**
//! via [`HttpTransport`]. That keeps synapse free of `reqwest::blocking` (and
//! free of the tokio "runtime inside a runtime" panic that a nested executor
//! would cause on a worker thread) — the host that runs the module (e.g. the
//! 端脑 `call_wasm`, which is already off the async runtime) supplies a
//! transport it can safely block on.
//!
//! # The gate (the whole point)
//! [`guard_url`] rejects, before any socket is opened:
//! - any scheme other than `http`/`https`;
//! - a missing/empty host;
//! - `localhost`, `.localhost`, `.local`, `.internal` names;
//! - **every** IP address that is loopback / private (RFC1918) / link-local /
//!   multicast / unspecified / the CGNAT & doc & benchmark ranges;
//! - for a hostname: it is resolved and **every** resolved address must pass
//!   the IP test (fail closed — if any answer is private we reject, so a
//!   rebinding-to-internal DNS record does not slip through).
//!
//! ⚠️ Known residual (documented, not silent): resolving to pre-flight the
//! address and then letting the caller's transport resolve again is a
//! TOCTOU window a hostile DNS could race. Closing it needs the transport to
//! connect to the *pre-resolved* IP (pinning) — deferred to P4b where the
//! concrete transport lands. Until then, only trusted wasm packages should
//! be installed (same posture as the sha256-verified binary store).
//!
//! # The ABI
//! `env.http_request(req_ptr: i32, req_len: i32, resp_ptr: i32, resp_cap: i32) -> i32`
//! - The module writes a request JSON at `req_ptr`:
//!   `{"method":"GET","url":"https://…","headers":[["k","v"],…],"body":""}`.
//! - The host writes a response JSON at `resp_ptr`:
//!   `{"status":200,"body":"…"}`, and returns the number of bytes written (≥0).
//! - Negative return = error code (see `E_*`), so a module can `br` on it:
//!   the sandbox never returns a half-written body under an error code.
//! - The module owns both buffers (it must have `memory.grow`n enough room);
//!   the host never grows wasm memory itself.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::Arc;

/// A single outbound request the wasm module asks the host to make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmHttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: String,
}

/// The host's reply, serialized back into wasm memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmHttpResponse {
    pub status: u16,
    #[serde(default)]
    pub body: String,
}

/// Why a request was refused before any socket opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    /// scheme not http/https (or unparseable authority)
    DisallowedScheme,
    MissingHost,
    /// literal or resolved to loopback/private/link-local/…
    LocalOrPrivate,
    /// hostname did not resolve to any usable address
    Resolve,
}

/// Error codes returned to the module (negative i32; the ABI contract).
pub const E_NO_TRANSPORT: i32 = -1;
pub const E_REQ_TOO_LARGE: i32 = -2;
pub const E_BAD_REQUEST_JSON: i32 = -3;
pub const E_DISALLOWED_SCHEME: i32 = -4;
pub const E_MISSING_HOST: i32 = -5;
pub const E_LOCAL_OR_PRIVATE: i32 = -6;
pub const E_RESOLVE: i32 = -7;
pub const E_TOO_MANY: i32 = -8;
pub const E_TRANSPORT: i32 = -9;
pub const E_RESP_TOO_SMALL: i32 = -10;
pub const E_NO_MEMORY: i32 = -11;
pub const E_OOB: i32 = -12;

/// The transport the caller supplies. Implementations may block (the host
/// call runs off the async runtime — see module docs). The `max_response_bytes`
/// argument is the policy cap; a conforming transport must not return a body
/// longer than it.
pub trait HttpTransport: Send + Sync {
    fn request(
        &self,
        req: &WasmHttpRequest,
        max_response_bytes: usize,
    ) -> Result<WasmHttpResponse, String>;

    /// The policy the host wants this transport to run under.
    ///
    /// The runtime uses it for the **ABI-side** gate as well, so a host
    /// that detected a fake-IP resolver can relax exactly that range
    /// (see [`HttpPolicy::allow_fakeip_range`]) without the module
    /// ever seeing a policy.
    fn policy(&self) -> HttpPolicy {
        HttpPolicy::default()
    }
}

/// Per-execution budget for the host capability.
#[derive(Debug, Clone)]
pub struct HttpPolicy {
    pub max_requests: u32,
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    /// Allow the IETF benchmarking range (198.18.0.0/15) as a target.
    ///
    /// ⚠️ **Why this knob exists**: local proxy stacks in fake-IP mode
    /// (Clash/mihomo and friends) resolve **every** hostname into
    /// 198.18.0.0/15 and route those addresses through the proxy. A
    /// strict guard therefore refuses *every* domain on such a machine
    /// ⇒ the whole capability is unusable there.
    ///
    /// ⚠️ **What it costs**: under fake-IP DNS, a name that points at an
    /// internal host is indistinguishable from a public one (both land in
    /// 198.18/15) ⇒ with this knob on, **name-based** SSRF is no longer
    /// caught; only *literal* private/loopback targets still are. ⇒ The
    /// host must enable it **only** after detecting a fake-IP resolver
    /// (and should rely on the user's own proxy policy for the rest).
    /// Default: **off** (strict).
    pub allow_fakeip_range: bool,
}

impl Default for HttpPolicy {
    fn default() -> Self {
        // A handful of API calls per JSON-RPC turn; request bodies are tiny
        // JSON; 256 KiB caps a tool response while staying far under the
        // module's 1 MiB addressable ceiling (16 pages).
        Self {
            max_requests: 8,
            max_request_bytes: 32 * 1024,
            max_response_bytes: 256 * 1024,
            allow_fakeip_range: false,
        }
    }
}

/// Bundle a transport + policy handed to [`WasmRuntime::execute_with_http`].
pub struct HttpHostConfig {
    pub transport: Arc<dyn HttpTransport>,
    pub policy: HttpPolicy,
}

/// Store state carried through one execution: the transport, the budget, and
/// a request counter.
pub struct HttpHostState {
    pub transport: Option<Arc<dyn HttpTransport>>,
    pub policy: HttpPolicy,
    pub used: u32,
}

/// True if this IP must never be reached by a wasm module (strict
/// posture — the benchmarking range included).
pub fn ip_is_disallowed(ip: IpAddr) -> bool {
    ip_is_disallowed_with(ip, false)
}

/// Same, but with the fake-IP exception applied (see
/// [`HttpPolicy::allow_fakeip_range`]).
pub fn ip_is_disallowed_with(ip: IpAddr, allow_fakeip_range: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_unspecified()
                // 0.0.0.0/8 "this network"
                || o[0] == 0
                // 100.64.0.0/10 CGNAT
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
                // 192.0.0.0/24 IETF protocol assignments
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24 doc ranges
                || (o[0] == 192 && o[1] == 0 && o[2] == 2)
                || (o[0] == 198 && o[1] == 51 && o[2] == 100)
                || (o[0] == 203 && o[1] == 0 && o[2] == 113)
                // 198.18.0.0/15 benchmarking — the range fake-IP proxy
                // stacks map every hostname into. Rejected by default;
                // only a host that detected such a resolver may relax it.
                || (o[0] == 198 && (o[1] & 0xfe) == 18 && !allow_fakeip_range)
                // 240.0.0.0/4 reserved + 255 broadcast (is_multicast covers ff)
                || o[0] >= 240
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                // ::ffff:a.b.c.d IPv4-mapped → recheck the embedded v4
                || v6.segments()[..5] == [0, 0, 0, 0, 0] && {
                    let s = v6.segments();
                    if s[5] == 0xffff {
                        let ip = IpAddr::from([
                            (s[6] >> 8) as u8,
                            (s[6] & 0xff) as u8,
                            (s[7] >> 8) as u8,
                            (s[7] & 0xff) as u8,
                        ]);
                        matches!(ip, IpAddr::V4(v4) if ip_is_disallowed_with(IpAddr::V4(v4), allow_fakeip_range))
                    } else {
                        false
                    }
                }
        }
    }
}

/// Split `scheme://authority` and return `(scheme, host)`. No external URL
/// crate (synapse's dep set stays as-is); this parses the subset a module is
/// allowed to build.
fn split_url(raw: &str) -> Option<(String, String)> {
    let (scheme, rest) = raw.split_once("://")?;
    // authority ends at first '/', '?' or '#'
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(rest);
    // strip userinfo (everything up to the last '@')
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    if hostport.is_empty() {
        return None;
    }
    let host = if let Some(bracket) = hostport.strip_prefix('[') {
        // IPv6 literal [::1]:port
        bracket.split(']').next()?
    } else {
        // host or host:port — drop the port
        hostport.rsplit_once(':').map(|(h, _)| h).unwrap_or(hostport)
    };
    Some((scheme.to_ascii_lowercase(), host.to_ascii_lowercase()))
}

fn guard_host(host: &str, allow_fakeip_range: bool) -> Result<(), GuardError> {
    if host.is_empty() {
        return Err(GuardError::MissingHost);
    }
    // disallow reserved/special-use names outright
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return Err(GuardError::LocalOrPrivate);
    }
    // literal IP?
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if ip_is_disallowed_with(ip, allow_fakeip_range) {
            Err(GuardError::LocalOrPrivate)
        } else {
            Ok(())
        };
    }
    // ⚠️ fake-IP mode: the **proxy** resolves the name, not us. Our
    // resolver's answers would be meaningless (every name lands in
    // 198.18/15), and resolving here would both depend on DNS and
    // refuse every public name. ⇒ In that mode we do not resolve at
    // all; literal private/loopback targets and the special names
    // above stay refused, and the user's own proxy policy is what
    // protects the LAN (see `HttpPolicy::allow_fakeip_range`).
    if allow_fakeip_range {
        return Ok(());
    }
    // else resolve and fail closed if any answer is disallowed
    let addrs: Vec<IpAddr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|_| GuardError::Resolve)?
        .map(|sa| sa.ip())
        .collect();
    if addrs.is_empty() {
        return Err(GuardError::Resolve);
    }
    if addrs
        .iter()
        .any(|ip| ip_is_disallowed_with(*ip, allow_fakeip_range))
    {
        return Err(GuardError::LocalOrPrivate);
    }
    Ok(())
}

/// The gate. Rejects disallowed schemes + local/private targets before any
/// socket opens (see module docs).
pub fn guard_url(raw: &str) -> Result<(), GuardError> {
    guard_url_with(raw, &HttpPolicy::default())
}

/// The gate, with the host's policy (the fake-IP exception lives there).
pub fn guard_url_with(raw: &str, policy: &HttpPolicy) -> Result<(), GuardError> {
    let (scheme, host) = split_url(raw).ok_or(GuardError::DisallowedScheme)?;
    if scheme != "http" && scheme != "https" {
        return Err(GuardError::DisallowedScheme);
    }
    guard_host(&host, policy.allow_fakeip_range)
}

fn guard_error_code(e: GuardError) -> i32 {
    match e {
        GuardError::DisallowedScheme => E_DISALLOWED_SCHEME,
        GuardError::MissingHost => E_MISSING_HOST,
        GuardError::LocalOrPrivate => E_LOCAL_OR_PRIVATE,
        GuardError::Resolve => E_RESOLVE,
    }
}

/// The whole decision path for one request, as a **pure function** over bytes
/// (no wasmtime types) so every branch is unit-testable without a module.
/// The runtime's linker closure does the wasm-memory read/write around it.
///
/// Returns the response JSON to write into wasm memory (Ok), or an `E_*` code
/// (Err). Never mutates `used` — the caller increments only on `Ok`.
pub fn process_http_request(
    transport: &Arc<dyn HttpTransport>,
    policy: &HttpPolicy,
    used: u32,
    req_bytes: &[u8],
) -> Result<Vec<u8>, i32> {
    if req_bytes.is_empty() || req_bytes.len() > policy.max_request_bytes {
        return Err(E_REQ_TOO_LARGE);
    }
    let req: WasmHttpRequest =
        serde_json::from_slice(req_bytes).map_err(|_| E_BAD_REQUEST_JSON)?;
    guard_url_with(&req.url, policy).map_err(guard_error_code)?;
    if used >= policy.max_requests {
        return Err(E_TOO_MANY);
    }
    let resp = transport
        .request(&req, policy.max_response_bytes)
        .map_err(|_| E_TRANSPORT)?;
    serde_json::to_vec(&WasmHttpResponse {
        status: resp.status,
        body: resp.body,
    })
    .map_err(|_| E_TRANSPORT)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the gate (offline: only IP literals + special names, no DNS) ----

    #[test]
    fn public_ip_literals_pass() {
        assert_eq!(guard_url("https://93.184.216.34/"), Ok(()));
        assert_eq!(guard_url("http://8.8.8.8/x?y=1"), Ok(()));
        assert_eq!(guard_url("https://[2606:4700:4700::1111]/"), Ok(()));
    }

    #[test]
    fn only_http_and_https() {
        assert_eq!(guard_url("file:///etc/passwd"), Err(GuardError::DisallowedScheme));
        assert_eq!(guard_url("ftp://8.8.8.8/"), Err(GuardError::DisallowedScheme));
        assert_eq!(guard_url("gopher://8.8.8.8/"), Err(GuardError::DisallowedScheme));
        // no scheme at all
        assert_eq!(guard_url("8.8.8.8"), Err(GuardError::DisallowedScheme));
    }

    #[test]
    fn v4_local_private_and_reserved_rejected() {
        for u in [
            "http://127.0.0.1/",
            "http://10.1.2.3/",
            "http://172.16.5.4/",
            "http://192.168.0.7/",
            "http://169.254.1.1/",
            "http://0.0.0.0/",
            "http://100.64.0.1/",
            "http://192.0.2.1/",
            "http://198.18.0.1/",
            "http://224.0.0.1/",
            "http://255.255.255.255/",
            "http://240.0.0.1/",
        ] {
            assert_eq!(guard_url(u), Err(GuardError::LocalOrPrivate), "{u} must be rejected");
        }
    }

    #[test]
    fn v6_local_rejected() {
        for u in [
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[fe80::1]/",
            "http://[ff02::1]/",
            "http://[::]/",
        ] {
            assert_eq!(guard_url(u), Err(GuardError::LocalOrPrivate), "{u}");
        }
    }

    #[test]
    fn ipv4_mapped_v6_rejected_by_embedded_v4() {
        // ::ffff:127.0.0.1 → the embedded 127.0.0.1 must trip the v4 rule
        assert_eq!(
            guard_url("http://[::ffff:7f00:1]/"),
            Err(GuardError::LocalOrPrivate)
        );
    }

    #[test]
    fn special_names_rejected() {
        for u in [
            "http://localhost/x",
            "http://api.localhost/x",
            "http://printer.local/",
            "http://vault.internal/",
        ] {
            assert_eq!(guard_url(u), Err(GuardError::LocalOrPrivate), "{u}");
        }
    }

    #[test]
    fn userinfo_and_port_stripped_before_host_check() {
        // creds and port must not let `evil@127.0.0.1` slip past
        assert_eq!(
            guard_url("http://user:pass@127.0.0.1:8080/admin"),
            Err(GuardError::LocalOrPrivate)
        );
        // a public host with port + userinfo still passes on the host part
        assert_eq!(guard_url("https://8.8.8.8:443/q"), Ok(()));
    }

    #[test]
    fn empty_host_rejected() {
        assert_eq!(guard_url("http:///path"), Err(GuardError::DisallowedScheme));
    }

    #[test]
    fn the_benchmarking_range_is_refused_by_default() {
        // 198.18.0.0/15 is where fake-IP proxy stacks put every name.
        // Strict by default — a host that has *not* detected such a
        // resolver must not be able to reach it.
        assert_eq!(
            guard_url("http://198.18.0.70/"),
            Err(GuardError::LocalOrPrivate)
        );
        assert!(ip_is_disallowed("198.18.0.70".parse().unwrap()));
    }

    #[test]
    fn a_fakeip_aware_policy_relaxes_only_that_range() {
        let relaxed = HttpPolicy {
            allow_fakeip_range: true,
            ..Default::default()
        };
        // the fake-IP range is reachable now …
        assert_eq!(guard_url_with("http://198.18.0.70/", &relaxed), Ok(()));
        // … and **nothing else** is: literal loopback / RFC1918 stay out,
        // and the special names stay out.
        for u in [
            "http://127.0.0.1/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://localhost/",
            "http://[::1]/",
        ] {
            assert_eq!(
                guard_url_with(u, &relaxed),
                Err(GuardError::LocalOrPrivate),
                "{u} must stay refused"
            );
        }
    }

    #[test]
    fn fakeip_mode_does_not_resolve_names() {
        // In fake-IP mode the *proxy* resolves the name ⇒ our resolver's
        // answers would be meaningless. Proof that we do not depend on
        // DNS here: a name that cannot resolve at all is still allowed
        // (the proxy may know it), while the special names stay out.
        let relaxed = HttpPolicy {
            allow_fakeip_range: true,
            ..Default::default()
        };
        assert_eq!(
            guard_url_with("http://definitely-not-a-real-host.invalid/", &relaxed),
            Ok(())
        );
        assert_eq!(
            guard_url_with("http://api.localhost/", &relaxed),
            Err(GuardError::LocalOrPrivate)
        );
        // … and in the strict posture the same name is refused (the
        // *reason* depends on the resolver — a fake-IP resolver would
        // answer 198.18/15, a dead one would fail to resolve — so we
        // only pin "refused", not which refusal).
        assert!(guard_url("http://definitely-not-a-real-host.invalid/").is_err());
    }

    // ---- the full decision path (stub transport, no wasm, no DNS) ----

    struct StubTransport {
        calls: Arc<std::sync::atomic::AtomicU32>,
        status: u16,
        body: &'static str,
        fail: bool,
    }
    impl HttpTransport for StubTransport {
        fn request(
            &self,
            req: &WasmHttpRequest,
            _max: usize,
        ) -> Result<WasmHttpResponse, String> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail {
                return Err("transport down".into());
            }
            Ok(WasmHttpResponse {
                status: self.status,
                body: format!("{} {} {}", req.method, req.url, self.body),
            })
        }
    }

    /// A stub transport plus its call counter (the counter is handed out
    /// separately so the trait object can be passed as `&Arc<dyn …>` — a
    /// one-step coercion, which is what the API takes).
    fn stub(
        status: u16,
        body: &'static str,
    ) -> (Arc<std::sync::atomic::AtomicU32>, Arc<dyn HttpTransport>) {
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let t = Arc::new(StubTransport {
            calls: calls.clone(),
            status,
            body,
            fail: false,
        });
        (calls, t)
    }

    fn failing_stub() -> Arc<dyn HttpTransport> {
        Arc::new(StubTransport {
            calls: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            status: 0,
            body: "",
            fail: true,
        })
    }

    fn req_json(url: &str) -> Vec<u8> {
        serde_json::to_vec(&WasmHttpRequest {
            method: "GET".into(),
            url: url.into(),
            headers: vec![],
            body: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn guard_rejects_before_transport_is_touched() {
        let (calls, t) = stub(200, "x");
        let policy = HttpPolicy::default();
        let r = process_http_request(&t, &policy, 0, &req_json("http://127.0.0.1/"));
        assert_eq!(r.unwrap_err(), E_LOCAL_OR_PRIVATE);
        // ⚠️ the tooth: the transport must NOT have been called
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn public_url_calls_transport_once_and_serializes() {
        let (calls, t) = stub(200, "ok");
        let policy = HttpPolicy::default();
        let out = process_http_request(&t, &policy, 0, &req_json("https://93.184.216.34/"))
            .expect("public ip passes");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let parsed: WasmHttpResponse = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.status, 200);
        assert!(parsed.body.contains("93.184.216.34"));
    }

    #[test]
    fn bad_request_json_is_rejected() {
        let (calls, t) = stub(200, "");
        let r = process_http_request(&t, &HttpPolicy::default(), 0, b"not json");
        assert_eq!(r.unwrap_err(), E_BAD_REQUEST_JSON);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn budget_stops_after_max_requests() {
        let (calls, t) = stub(200, "");
        let policy = HttpPolicy { max_requests: 2, ..Default::default() };
        let body = req_json("https://8.8.8.8/");
        assert!(process_http_request(&t, &policy, 0, &body).is_ok());
        assert!(process_http_request(&t, &policy, 1, &body).is_ok());
        assert_eq!(
            process_http_request(&t, &policy, 2, &body).unwrap_err(),
            E_TOO_MANY
        );
        // the third was refused before touching the transport
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn transport_error_surfaces_as_code() {
        let t = failing_stub();
        assert_eq!(
            process_http_request(&t, &HttpPolicy::default(), 0, &req_json("https://8.8.8.8/"))
                .unwrap_err(),
            E_TRANSPORT
        );
    }

    #[test]
    fn oversize_request_is_rejected() {
        let (calls, t) = stub(200, "");
        let policy = HttpPolicy { max_request_bytes: 4, ..Default::default() };
        assert_eq!(
            process_http_request(&t, &policy, 0, &req_json("https://8.8.8.8/")).unwrap_err(),
            E_REQ_TOO_LARGE
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    // ---- end-to-end ABI: a real module calling env.http_request ----
    //
    // The module embeds the request JSON `{"method":"GET","url":"http://127.0.0.1/"}`
    // (exactly 42 bytes) at 2048, calls the host import once, and writes an
    // ASCII verdict at 1024 (`E6` = guard code -6, `E1` = no transport -1,
    // `OK` = the call went through) — `call_entry` reads the response back
    // from 1024 and requires it to be UTF-8 text, so the verdict has to be
    // text, not a raw byte.
    //
    // It pins three things at once: the linker binding exists, the guard runs
    // **before** the transport (no socket opened), and the plain `execute`
    // path keeps working for a module that imports the capability but has no
    // transport.

    const CALL_PRIVATE_HOST_WAT: &str = r#"
(module
  (import "env" "http_request" (func $http (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 2048) "{\"method\":\"GET\",\"url\":\"http://127.0.0.1/\"}")
  (func (export "handle") (param i32 i32) (result i32)
    (local $r i32)
    (local $m i32)
    (local.set $r (call $http (i32.const 2048) (i32.const 42) (i32.const 4096) (i32.const 512)))
    (if (i32.lt_s (local.get $r) (i32.const 0))
      (then
        (i32.store8 (i32.const 1024) (i32.const 69))
        (local.set $m (i32.sub (i32.const 0) (local.get $r))))
      (else
        (i32.store8 (i32.const 1024) (i32.const 79))
        (i32.store8 (i32.const 1025) (i32.const 75))
        (i32.store8 (i32.const 1026) (i32.const 10))
        (return (i32.const 3))))
    (i32.store8 (i32.const 1025) (i32.add (local.get $m) (i32.const 48)))
    (i32.store8 (i32.const 1026) (i32.const 10))
    (i32.const 3)))
"#;

    fn run_wat(wat: &str, with_http: bool) -> String {
        use super::super::runtime::WasmRuntime;
        let runtime = WasmRuntime::new();
        let module = runtime.load_module_from_bytes(&wat::parse_str(wat).unwrap()).unwrap();
        let exec = if with_http {
            runtime
                .execute_with_http(
                    &module,
                    "handle",
                    "ignored",
                    HttpHostConfig {
                        transport: Arc::new(StubTransport {
                            calls: Arc::new(std::sync::atomic::AtomicU32::new(0)),
                            status: 200,
                            body: "should never be reached",
                            fail: false,
                        }),
                        policy: HttpPolicy::default(),
                    },
                )
                .expect("module runs")
        } else {
            runtime.execute(&module, "handle", "ignored").expect("module runs")
        };
        exec.output.trim().to_string()
    }

    #[test]
    fn module_calling_private_host_gets_guard_code_not_a_socket() {
        // E6 = E_LOCAL_OR_PRIVATE (-6): the stub's body "should never be
        // reached" must NOT appear — the gate ran before the transport.
        assert_eq!(run_wat(CALL_PRIVATE_HOST_WAT, true), "E6");
    }

    #[test]
    fn plain_execute_keeps_linking_an_importing_module_without_transport() {
        // E1 = E_NO_TRANSPORT (-1) — the import is declared and answered,
        // and the old no-http path does not break.
        assert_eq!(run_wat(CALL_PRIVATE_HOST_WAT, false), "E1");
    }
}
