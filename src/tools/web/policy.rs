//! web 能力面的**门**：哪些 URL 允许被取。
//!
//! ⚠️ **为什么这一层必须存在**：引擎（obscura）自己的 SSRF 防线是
//! **全有全无**的 —— 它的逃生口 `OBSCURA_ALLOW_PRIVATE_NETWORK=1`
//! 会**整个**放开 deny-set（实测：含 `169.254.169.254` 云元数据，
//! 放开后请求真的走到了 socket 层）。⇒ ⇒ 引擎跑在宽松模式下时，
//! **唯一的细粒度防线就是这一层** ⇒ ⇒ 所以这一层必须比引擎严格。
//!
//! IP 判定**复用** `wasm::host_http::ip_is_disallowed_with`（同一份规则，
//! 别各写一套）—— 它已经算上 loopback／RFC1918／link-local／CGNAT／
//! 组播／文档段／benchmark 段。
//!
//! ⚠️ **fake-IP**：Clash/mihomo 之类的代理栈把**每个**名字映到
//! 198.18.0.0/15 ⇒ ⇒ 严格模式下这台机器**一个站都打不开**
//! （实测：`example.com` → 198.18.0.249 ⇒ 被判 forbidden）。
//! ⇒ ⇒ 所以本层**照端脑 `mcp::http_transport` 的口径**：先探测这台机器
//! 的解析器是不是 fake-IP，是就**只**放开 benchmark 段 ——
//! **字面**私网／环回／link-local 照拒；名字仍然要解析（解析到真私网
//! ⇒ 照拒）⇒ ⇒ 这比「fake-IP 模式下根本不解析」更严。

use std::net::{IpAddr, ToSocketAddrs};

use crate::wasm::host_http::ip_is_disallowed_with;

/// 探测器探针名（同端脑 `mcp::http_transport` 的口径）：一个到处都存在的名字。
const RESOLVER_PROBE_NAME: &str = "example.com";

/// 门没放行时的原因。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlPolicyError {
    #[error("scheme not allowed: {0}")]
    DisallowedScheme(String),
    #[error("missing host")]
    MissingHost,
    #[error("target is a local or private address")]
    LocalOrPrivate,
    #[error("host did not resolve")]
    Resolve,
}

/// 这台机器的解析器是不是 fake-IP（把每个名字映到 198.18.0.0/15）？
///
/// **探测**，不是假设 —— 端脑那条路也是这么做的。
pub fn detect_fakeip_resolver() -> bool {
    match (RESOLVER_PROBE_NAME, 0u16).to_socket_addrs() {
        Ok(addrs) => addrs.map(|sa| sa.ip()).any(is_benchmark_v4),
        Err(_) => false,
    }
}

/// 是不是 benchmark 段（198.18.0.0/15）里的 IPv4？
fn is_benchmark_v4(ip: IpAddr) -> bool {
    matches!(ip, IpAddr::V4(v4) if v4.octets()[0] == 198 && (v4.octets()[1] & 0xfe) == 18)
}

/// URL 的 host 部分（去掉 scheme／userinfo／port／path）。
pub fn host_of(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // userinfo 之后才是 host
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    // IPv6 字面量带方括号
    let host = if let Some(end) = authority.strip_prefix('[') {
        end.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() { None } else { Some(host) }
}

/// 门：这条 URL 允许去取吗？
///
/// `allow_fakeip_range` 只在**探测到 fake-IP 解析器**时为真
/// ⇒ ⇒ 它只放开 benchmark 段，别的一概不放。
pub fn check_url(url: &str, allow_fakeip_range: bool) -> Result<(), UrlPolicyError> {
    let (scheme, _) = url
        .split_once("://")
        .ok_or_else(|| UrlPolicyError::DisallowedScheme(url.to_string()))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(UrlPolicyError::DisallowedScheme(scheme));
    }
    let host = host_of(url).ok_or(UrlPolicyError::MissingHost)?;
    let lower = host.to_ascii_lowercase();
    // 特殊名字：它们要么是本机，要么是内网约定
    if lower == "localhost"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
    {
        return Err(UrlPolicyError::LocalOrPrivate);
    }
    // 字面 IP：直接判
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if ip_is_disallowed_with(ip, allow_fakeip_range) {
            Err(UrlPolicyError::LocalOrPrivate)
        } else {
            Ok(())
        };
    }
    // 名字：解析后**逐个答案**过门（fail closed）
    let addrs: Vec<IpAddr> = match (host, 0u16).to_socket_addrs() {
        Ok(it) => it.map(|sa| sa.ip()).collect(),
        Err(_) => return Err(UrlPolicyError::Resolve),
    };
    if addrs.is_empty() {
        return Err(UrlPolicyError::Resolve);
    }
    for ip in addrs {
        if ip_is_disallowed_with(ip, allow_fakeip_range) {
            return Err(UrlPolicyError::LocalOrPrivate);
        }
    }
    Ok(())
}
