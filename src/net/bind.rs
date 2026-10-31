//! 「**监听哪儿**」与「**公布哪儿**」—— 两个**不同**的问题。
//!
//! ## 为什么单独一个模块（2026-11-01）
//!
//! 之前的形状是**一个字段说两件事**：
//!
//! * 监听地址 = `0.0.0.0:19500` ⇒ 「监听所有网卡」；
//! * 公布地址 = `format!("http://{bind}")` ⇒ **同一个字符串**。
//!
//! ⇒ 而 `0.0.0.0` **不是一个可连接的地址** —— 它是「监听所有网卡」，
//! 不是一个「别人能用来连我」的地址 ⇒ ⇒ 单机上**碰巧**能通（本机把
//! `0.0.0.0` 当回环）⇒ ⇒ **单机开发里完全看不见这个洞**
//! ⇒ ⇒ 换台机器，`0.0.0.0` 指的是**那台机器自己的回环**
//! ⇒ ⇒ **注册成功、永远连不上** —— 又一个「失败长得像成功」。
//!
//! ⇒ 所以必须**两个槽**：**监听地址**（绑哪儿）与**公布地址**（我在哪儿）。
//!
//! ## 为什么放在 synapse（而不是端脑仓）
//!
//! 「**别各写一套**」—— 而**每个**叶区仓（杏仁核 / 颞叶 / 枕叶 / 顶叶）
//! 都依赖 **synapse**，却**没有**依赖端脑仓
//! ⇒ ⇒ 解析器放在端脑仓 ⇒ ⇒ 叶区就**用不上** ⇒ ⇒ 等于逼着每个叶区
//! 各写一份 ⇒ ⇒ **两份实现** ⇒ ⇒ 迟早漂。
//!
//! ⇒ ⇒ 端脑仓那份（`telencephalon::config::bind`）现在**委托**到这里，
//! **路径不变** ⇒ ⇒ 已有调用点零改动。

/// 默认回环地址。
pub const LOOPBACK: &str = "127.0.0.1";

/// 解析出来的监听目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindTarget {
    /// 监听主机。`"0.0.0.0"` = 宽绑定（**只在显式配置时才会出现**）。
    pub host: String,
    /// 监听端口。
    pub port: u16,
}

impl BindTarget {
    /// `host:port`（给 `TcpListener::bind` 用）。
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// 这是一次**宽绑定**吗（监听所有网卡 ⇒ 局域网可达）。
    pub fn is_wide(&self) -> bool {
        self.host == "0.0.0.0" || self.host == "::" || self.host == "[::]"
    }

    /// 自调用（自己打自己）该用的 base URL。
    ///
    /// ⚠️ 宽绑定（`0.0.0.0`）在自调用时必须换成**回环** —— `0.0.0.0`
    /// 是「监听所有网卡」，不是「一个可连接的地址」（在 Windows 上连
    /// `0.0.0.0` 能不能通取决于实现，不能依赖）。
    pub fn self_call_base_url(&self) -> String {
        let host = if self.host.is_empty() || self.is_wide() {
            LOOPBACK
        } else {
            self.host.as_str()
        };
        format!("http://{host}:{}", self.port)
    }

    /// **公布地址** —— 「我在哪儿，别人怎么连我」。
    ///
    /// 规则（**一条共享规则**，不各写一套）：
    ///
    /// 1. **显式给了就用它**（原样，不加工 —— scheme / 路径 / 端口由
    ///    部署自己决定）；
    /// 2. **没给 ⇒ 宽绑定回落回环**（`0.0.0.0` 不是可连接的地址 ⇒
    ///    拿它当公布地址 = 「注册成功、永远连不上」）；
    /// 3. 没给 + 不是宽绑定 ⇒ 用监听地址。
    pub fn public_url(&self, explicit: Option<&str>) -> String {
        if let Some(e) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
            return e.to_string();
        }
        let host = if self.host.is_empty() || self.is_wide() {
            LOOPBACK
        } else {
            self.host.as_str()
        };
        format!("http://{host}:{}", self.port)
    }
}

/// 解析监听地址的**原始字符串**。
///
/// 容错：空 / 裸端口 / `:端口` / `host:port` / `[v6]:port` 都认；
/// 认不出来 ⇒ **回环 + 默认端口**。
pub fn parse_bind(raw: &str, default_port: u16) -> BindTarget {
    let raw = raw.trim();
    if raw.is_empty() {
        return BindTarget {
            host: LOOPBACK.to_string(),
            port: default_port,
        };
    }

    // IPv6 字面量（带方括号）："[::1]:60080"
    if raw.starts_with('[') {
        if let Some(close) = raw.find(']') {
            let host = raw[1..close].to_string();
            let rest = raw[close + 1..].trim_start_matches(':');
            let port = match rest.parse::<u16>() {
                Ok(p) => p,
                Err(_) => {
                    tracing::warn!(
                        raw = %raw,
                        "bind address has an unparsable port — falling back to the default port"
                    );
                    default_port
                }
            };
            return BindTarget { host, port };
        }
    }

    // "host:port" —— 用**最后一个**冒号切（IPv6 不带方括号时也切对）。
    if let Some(idx) = raw.rfind(':') {
        let host = raw[..idx].trim();
        let port_str = raw[idx + 1..].trim();
        let port = match port_str.parse::<u16>() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    raw = %raw,
                    "bind address has an unparsable port — falling back to the default port"
                );
                default_port
            }
        };
        let host = if host.is_empty() { LOOPBACK } else { host };
        return BindTarget {
            host: host.to_string(),
            port,
        };
    }

    // 裸端口："60080" —— 老配置里有人这么写。
    if let Ok(port) = raw.parse::<u16>() {
        return BindTarget {
            host: LOOPBACK.to_string(),
            port,
        };
    }

    // ⚠️ **静默回落是这条线上最贵的失败模式**（它长得像成功）⇒ ⇒ 必须喊一声。
    tracing::warn!(
        raw = %raw,
        "bind address is not a host:port — falling back to loopback"
    );
    BindTarget {
        host: LOOPBACK.to_string(),
        port: default_port,
    }
}

/// **一次解出两个地址** —— 叶区侧只需要这一个调用。
///
/// * `raw_bind`：监听地址（`LOBE_BIND` 那一类环境变量）；
/// * `raw_public`：**显式**的公布地址（`LOBE_PUBLIC_URL` 那一类）。
///
/// ⇒ 返回 `(监听目标, 公布地址)` ⇒ ⇒ 监听用第一个、**公布用第二个**
/// ⇒ ⇒ 「绑哪儿」与「我在哪儿」从此**不是一个字段**。
pub fn resolve_listen_and_public(
    raw_bind: &str,
    default_port: u16,
    raw_public: Option<&str>,
) -> (BindTarget, String) {
    let target = parse_bind(raw_bind, default_port);
    let public = target.public_url(raw_public);
    (target, public)
}

/// **洗一遍**「报上来的地址」—— 让它变成**可连接的**地址。
///
/// ⚠️ 宽绑定（`0.0.0.0` / `::`）**永远不是一个可连接的地址**
/// ⇒ ⇒ 洗成回环 ⇒ ⇒ 单机上碰巧能通、跨机永远连不上。
///
/// ⇒ ⇒ **谁报上来的都洗**（叶区报的、配置里写的）⇒ ⇒ **一条规则**。
pub fn sanitize_public_url(url: &str) -> String {
    let raw = url.trim();
    if raw.is_empty() {
        return raw.to_string();
    }

    // scheme
    let (scheme, rest) = match raw.find("://") {
        Some(i) => (&raw[..i], &raw[i + 3..]),
        None => ("http", raw),
    };

    // authority / path
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };

    // host[:port]（IPv6 带方括号）
    let (host, port) = if let Some(stripped) = authority.strip_prefix('[') {
        match stripped.find(']') {
            Some(i) => {
                let host = &stripped[..i];
                let after = &stripped[i + 1..];
                let port = after.strip_prefix(':').map(|p| p.to_string());
                (host.to_string(), port)
            }
            None => (authority.to_string(), None),
        }
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
                (h.to_string(), Some(p.to_string()))
            }
            _ => (authority.to_string(), None),
        }
    };

    let host = if is_wide_host(&host) {
        LOOPBACK.to_string()
    } else {
        host
    };
    let port_part = port.map(|p| format!(":{p}")).unwrap_or_default();
    format!("{scheme}://{host}{port_part}{path}")
}

/// 宽绑定的主机名？（`0.0.0.0` / `::` / `[::]`）
fn is_wide_host(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "::" | "[::]")
}

/// 从「基地址 URL」推导**公布地址**（「我在哪儿，别人怎么连我」）。
///
/// * **显式给了就用它**（原样 ⇒ scheme / 路径 / 端口由部署自己决定）；
/// * 没给 ⇒ 把基地址**洗成可连接的**（宽绑定 ⇒ 回环）。
///
/// ⚠️ 「显式原样」只在**叶区侧**成立 ⇒ ⇒ 端脑侧还会再洗一遍
/// （信任边界上只认**可连接**的地址）。
pub fn public_url_for_base(base_url: &str, explicit: Option<&str>) -> String {
    if let Some(e) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_string();
    }
    sanitize_public_url(base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_binds_to_loopback_not_to_the_wide_address() {
        let t = parse_bind("", 60080);
        assert_eq!(t.host, LOOPBACK);
        assert_eq!(t.port, 60080);
        assert!(!t.is_wide());
    }

    #[test]
    fn the_configured_port_is_actually_used() {
        assert_eq!(parse_bind("127.0.0.1:61234", 60080).port, 61234);
    }

    #[test]
    fn a_bare_port_means_loopback_on_that_port() {
        let t = parse_bind("60080", 60080);
        assert_eq!(t.host, LOOPBACK);
        assert_eq!(t.port, 60080);
    }

    #[test]
    fn an_explicit_wide_bind_stays_wide() {
        let t = parse_bind("0.0.0.0:60080", 60080);
        assert!(t.is_wide());
    }

    #[test]
    fn self_calls_never_target_the_wildcard_address() {
        assert_eq!(
            parse_bind("0.0.0.0:60080", 60080).self_call_base_url(),
            "http://127.0.0.1:60080"
        );
        assert_eq!(
            parse_bind("192.168.1.20:60080", 60080).self_call_base_url(),
            "http://192.168.1.20:60080"
        );
    }

    #[test]
    fn garbage_falls_back_to_loopback_instead_of_guessing() {
        let t = parse_bind("not-an-address", 60080);
        assert_eq!(t.host, LOOPBACK);
        assert_eq!(t.port, 60080);
    }

    #[test]
    fn ipv6_literals_are_understood() {
        assert_eq!(parse_bind("[::1]:60080", 60080).host, "::1");
    }

    /// ⚠️⚠️ **核心那条**：宽绑定的**公布地址**不许是 `0.0.0.0`。
    ///
    /// ⇒ ⇒ 那不是「可连接的地址」⇒ ⇒ 单机上碰巧能通 ⇒ ⇒ 换台机器
    /// ⇒ ⇒ **注册成功、永远连不上**。
    #[test]
    fn a_wide_bind_never_publishes_the_wildcard_address() {
        let wide = parse_bind("0.0.0.0:19500", 19500);
        assert_eq!(wide.public_url(None), "http://127.0.0.1:19500");
    }

    #[test]
    fn an_explicit_public_url_wins_and_is_taken_verbatim() {
        let wide = parse_bind("0.0.0.0:19500", 19500);
        assert_eq!(
            wide.public_url(Some("http://192.168.1.20:19500")),
            "http://192.168.1.20:19500"
        );
        // 原样：不替它加 scheme、不改端口。
        assert_eq!(
            wide.public_url(Some("https://brain.example/lobes")),
            "https://brain.example/lobes"
        );
    }

    #[test]
    fn a_blank_public_url_counts_as_not_given() {
        let wide = parse_bind("0.0.0.0:19500", 19500);
        assert_eq!(wide.public_url(Some("   ")), "http://127.0.0.1:19500");
    }

    #[test]
    fn a_loopback_bind_publishes_itself() {
        let t = parse_bind("127.0.0.1:9500", 9500);
        assert_eq!(t.public_url(None), "http://127.0.0.1:9500");
    }

    /// ✅ **一次解出两个** —— 叶区侧只要这一个调用。
    #[test]
    fn listening_and_publishing_are_resolved_together() {
        let (listen, public) = resolve_listen_and_public("0.0.0.0:19500", 19500, None);
        assert_eq!(listen.socket_addr(), "0.0.0.0:19500");
        assert_eq!(public, "http://127.0.0.1:19500");

        let (listen, public) =
            resolve_listen_and_public("0.0.0.0:19500", 19500, Some("http://192.168.1.20:19500"));
        assert_eq!(listen.socket_addr(), "0.0.0.0:19500");
        assert_eq!(public, "http://192.168.1.20:19500");
    }

    /// ⚠️⚠️ **核心那条的第二面**：宽绑定的**基地址**洗成回环。
    ///
    /// ⇒ ⇒ 公布地址**永远不是** `0.0.0.0`。
    #[test]
    fn a_wide_base_url_is_washed_into_loopback() {
        assert_eq!(
            sanitize_public_url("http://0.0.0.0:9500"),
            "http://127.0.0.1:9500"
        );
        assert_eq!(
            sanitize_public_url("http://[::]:9500"),
            "http://127.0.0.1:9500"
        );
    }

    #[test]
    fn a_connectable_base_url_is_left_alone() {
        assert_eq!(
            sanitize_public_url("http://192.168.1.20:9500"),
            "http://192.168.1.20:9500"
        );
        // 路径原样留着。
        assert_eq!(
            sanitize_public_url("https://brain.example:8443/lobes"),
            "https://brain.example:8443/lobes"
        );
    }

    #[test]
    fn an_explicit_public_url_is_taken_verbatim() {
        assert_eq!(
            public_url_for_base("http://0.0.0.0:9500", Some("http://192.168.1.20:9500")),
            "http://192.168.1.20:9500"
        );
        // 原样：不替它加 scheme、不改端口。
        assert_eq!(
            public_url_for_base("http://0.0.0.0:9500", Some("https://brain.example/lobes")),
            "https://brain.example/lobes"
        );
    }

    #[test]
    fn a_blank_explicit_public_url_counts_as_not_given() {
        assert_eq!(
            public_url_for_base("http://0.0.0.0:9500", Some("   ")),
            "http://127.0.0.1:9500"
        );
    }
}
