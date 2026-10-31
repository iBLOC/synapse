//! Helper to register a native lobe with Telencephalon (L3.5).
//!
//! POSTs to `{encephalon_url}/telencephalon/lobes/{name}` with the lobe's
//! AgentCard. Best-effort: on failure, log a warning. The lobe
//! should still work standalone when ENCEPHALON_URL is unset.
//!
//! ## ⚠️ 两个槽：「监听地址」与「公布地址」
//!
//! 请求体带**两个**地址：
//!
//! * `url` —— **监听地址**（「我绑哪儿」）⇒ ⇒ 宽绑定时它是 `0.0.0.0`
//!   ⇒ ⇒ 那**不是一个可连接的地址**；
//! * `public_url` —— **公布地址**（「别人怎么连我」）⇒ ⇒ 端脑连叶区时
//!   **优先**用它 ⇒ ⇒ 跨机部署的命根子。
//!
//! ⇒ ⇒ 规则**一份**（`synapse::net::bind`）：显式给了就用它；没给 ⇒ 把
//! 监听地址**洗成可连接的**（宽绑定 ⇒ 回环）。
//!
//! ## ⚠️ 路径：先新后旧（2026-11-01）
//!
//! 端脑 v0.3 改名（`cortex` → `encephalon` → `telencephalon`）时，
//! **共享库没跟着改** ⇒ ⇒ 这个函数一直打在**老路径**
//! `/encephalon/lobes/{name}` ⇒ ⇒ 而端脑只挂
//! `/telencephalon/lobes/{name}` ⇒ ⇒ **404** ⇒ ⇒ 注册**静默失败**
//! （只有一行 `warn!`）⇒ ⇒ 颞叶 / 顶叶 / 枕叶 / 前额叶**从来没注册成功过**
//! ⇒ ⇒ 端脑「知道自己有哪些叶区」有一半是自己种的占位。
//!
//! ⇒ ⇒ 现在：**先打新路径**；**只有 404** 才回落到老路径一次
//! ⇒ ⇒（401 / 403 **不**回落 ⇒ 不掩盖鉴权问题）。
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

use serde_json::Value;

use crate::lobe::AgentCard;
use crate::net::bind::public_url_for_base;

/// 现代路径（端脑 v0.3+）。
const MODERN_PATH: &str = "/telencephalon/lobes";
/// 老路径（端脑 v0.2 时代）—— 老部署只挂这个 ⇒ 回落到它一次。
const LEGACY_PATH: &str = "/encephalon/lobes";

/// 注册失败的原因（**只有 404 才值得回落**）。
enum RegisterFailure {
    /// 路由不存在 ⇒ 可能是个只挂老路径的老部署。
    NotFound,
    /// 别的失败（连不上 / 401 / 500 …）⇒ 不回落，直接报。
    Other(String),
}

impl std::fmt::Display for RegisterFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterFailure::NotFound => write!(f, "404 not found"),
            RegisterFailure::Other(m) => write!(f, "{m}"),
        }
    }
}

/// Register a native lobe with the telencephalon.
///
/// **公布地址**从两处来（优先级高的在前）：
/// 1. `LOBE_PUBLIC_URL` 环境变量（**显式** ⇒ ⇒ 跨机部署必须给这个）；
/// 2. 叶区自己的监听地址（洗成可连接的 ⇒ 宽绑定 ⇒ 回环）。
pub async fn register_with_encephalon(
    encephalon_url: &str,
    card: &AgentCard,
) -> Result<(), String> {
    let explicit = std::env::var("LOBE_PUBLIC_URL").ok();
    register_with_encephalon_public(encephalon_url, card, explicit.as_deref()).await
}

/// 同上，但**公布地址由调用方给**（不给 ⇒ 从监听地址推导）。
pub async fn register_with_encephalon_public(
    encephalon_url: &str,
    card: &AgentCard,
    public_url: Option<&str>,
) -> Result<(), String> {
    let base = encephalon_url.trim_end_matches('/');
    if base.is_empty() {
        return Err("register_with_encephalon: empty base url".to_string());
    }
    let public = public_url_for_base(&card.url, public_url);
    let body = serde_json::json!({
        "kind": "native",
        "url": card.url,
        "public_url": public,
        "agent_card": card,
    });

    match post_json(&format!("{base}{MODERN_PATH}/{}", card.name), &body).await {
        Ok(()) => Ok(()),
        Err(RegisterFailure::NotFound) => {
            post_json(&format!("{base}{LEGACY_PATH}/{}", card.name), &body)
                .await
                .map_err(|e| {
                    format!("register_with_encephalon failed (modern path 404, legacy path failed too): {e}")
                })
        }
        Err(RegisterFailure::Other(msg)) => {
            Err(format!("register_with_encephalon failed: {msg}"))
        }
    }
}

async fn post_json(url: &str, body: &Value) -> Result<(), RegisterFailure> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| RegisterFailure::Other(format!("reqwest: {e}")))?;
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| RegisterFailure::Other(format!("send: {e}")))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(RegisterFailure::NotFound);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(RegisterFailure::Other(format!("{status} {txt}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::post;
    use std::sync::{Arc, Mutex};

    fn card(url: &str) -> AgentCard {
        AgentCard::new("temporal", "test agent", url, "1.0.0")
    }

    /// 起一个**只挂指定路由**的假端脑，并记录收到的请求体。
    async fn spawn_fake_brain(
        mount: &'static str,
    ) -> (String, Arc<Mutex<Vec<Value>>>) {
        let seen: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let router: Router = Router::new().route(
            mount,
            post(move |axum::Json(body): axum::Json<Value>| {
                let sink = Arc::clone(&sink);
                async move {
                    sink.lock().unwrap().push(body);
                    axum::http::StatusCode::OK
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        (format!("http://{addr}"), seen)
    }

    #[test]
    fn register_with_encephalon_empty_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = card("http://localhost:8090");
        let result = rt.block_on(register_with_encephalon("", &card));
        assert!(result.is_err());
    }

    #[test]
    fn register_with_encephalon_invalid_url_fails() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let card = card("http://localhost:8090");
        let result = rt.block_on(register_with_encephalon("not-a-url", &card));
        assert!(result.is_err());
    }

    /// ⚠️⚠️ **这条是「改名收口漏了共享库」的回归钉子**：
    /// 端脑只挂**新路径** ⇒ ⇒ 打新路径必须**成功**。
    ///
    /// ⇒ ⇒ 修之前：404 ⇒ 回落到老路径 ⇒ 老路径也没有 ⇒ **Err**
    /// ⇒ ⇒ 而叶区只 `warn!` 一行 ⇒ ⇒ **注册静默失败**。
    #[test]
    fn the_modern_path_is_the_one_we_talk_to() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (base, seen) = spawn_fake_brain("/telencephalon/lobes/{name}").await;
            let card = card("http://127.0.0.1:8090");
            let result = register_with_encephalon(&base, &card).await;
            assert!(result.is_ok(), "modern path registration failed: {result:?}");
            let bodies = seen.lock().unwrap();
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0]["kind"], "native");
            assert_eq!(bodies[0]["url"], "http://127.0.0.1:8090");
        });
    }

    /// 老部署（只挂老路径）⇒ 回落到它 ⇒ 仍要成功。
    #[test]
    fn an_old_deployment_is_still_reachable_via_the_legacy_path() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (base, seen) = spawn_fake_brain("/encephalon/lobes/{name}").await;
            let card = card("http://127.0.0.1:8090");
            let result = register_with_encephalon(&base, &card).await;
            assert!(result.is_ok(), "legacy fallback failed: {result:?}");
            assert_eq!(seen.lock().unwrap().len(), 1);
        });
    }

    /// ⚠️ 两条路径都不存在 ⇒ 必须**报错**（不许静默成功）。
    #[test]
    fn no_registration_route_at_all_fails_loudly() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (base, _seen) = spawn_fake_brain("/somewhere/else/{name}").await;
            let card = card("http://127.0.0.1:8090");
            let result = register_with_encephalon(&base, &card).await;
            assert!(result.is_err(), "registration must fail loudly: {result:?}");
        });
    }

    /// ⚠️ 宽绑定**不许**被公布成 `0.0.0.0`。
    #[test]
    fn a_wide_bind_is_never_published_as_the_wildcard_address() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (base, seen) = spawn_fake_brain("/telencephalon/lobes/{name}").await;
            let card = card("http://0.0.0.0:9500");
            let result = register_with_encephalon_public(&base, &card, None).await;
            assert!(result.is_ok(), "registration failed: {result:?}");
            let bodies = seen.lock().unwrap();
            assert_eq!(bodies[0]["public_url"], "http://127.0.0.1:9500");
        });
    }

    /// 显式公布地址原样送出（跨机部署靠它）。
    #[test]
    fn an_explicit_public_url_is_sent_verbatim() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (base, seen) = spawn_fake_brain("/telencephalon/lobes/{name}").await;
            let card = card("http://0.0.0.0:9500");
            let result = register_with_encephalon_public(
                &base,
                &card,
                Some("http://192.168.1.20:9500"),
            )
            .await;
            assert!(result.is_ok(), "registration failed: {result:?}");
            let bodies = seen.lock().unwrap();
            assert_eq!(bodies[0]["public_url"], "http://192.168.1.20:9500");
        });
    }
}
