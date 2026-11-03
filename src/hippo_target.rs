//! 海马体放在哪儿 —— 一个进程里，「本地海马体」与「远端海马体」二选一。
//!
//! ## 为什么它存在
//!
//! 分布式计算（场景二 / 场景四）：备用机上**不跑**海马体 ⇒ ⇒ 它的
//! 记忆读写要走到主力机那**一个**海马体 ⇒ ⇒ 地址必须能被**显式**指定。
//!
//! ⚠️ **为什么不能只认环境变量** —— Android 进程**设不了环境变量**
//! ⇒ ⇒ 移动端只能走显式 setter ⇒ ⇒ 所以「显式覆盖」是这条规则里
//! **最响**的那一格。
//!
//! ## 优先级（一条规则，所有调用点一致）
//!
//! 1. **显式覆盖**（[`set_base_url`]）—— 宿主给的，最响。
//! 2. `HIPPOCAMPUS_HOST` 环境变量。
//! 3. `${TELENCEPHALON_PUBLIC_URL}` / `${CORTEX_PUBLIC_URL}` ＋ `/hippocampus`。
//! 4. `http://localhost:60080/hippocampus`（本机）。
//!
//! ## 为什么地址可以带路径前缀
//!
//! 远端海马体经**零信任代理**可达 ⇒ ⇒ 地址长这样：
//! `http://127.0.0.1:60081/_zt/<service>/hippocampus`
//! ⇒ ⇒ 代理按服务名路由并**去掉前缀** ⇒ ⇒ 上游收到的是
//! `/hippocampus/api/memory` ⇒ ⇒ 所以这里**不能**把路径砍掉。
//!
//! ## 为什么住在 synapse
//!
//! 端脑（telencephalon）、前额叶（prefrontal）、桌面端、移动端都依赖
//! synapse ⇒ ⇒ 「海马体在哪儿」这条规则只有一份 ⇒ ⇒ 谁也不会自己
//! 再抄一遍优先级。

use std::sync::{OnceLock, RwLock};

/// 显式覆盖 —— 宿主给的地址。
static OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn slot() -> &'static RwLock<Option<String>> {
    OVERRIDE.get_or_init(|| RwLock::new(None))
}

/// 显式指定海马体的 base URL（**不带**尾斜杠、不带 `/api/...`）。
///
/// - `Some(url)` ⇒ 用它（空的按 `None` 算）。
/// - `None` ⇒ 交回环境变量那条路。
pub fn set_base_url(url: Option<String>) {
    let normalized = url.map(|u| normalize(&u)).filter(|u| !u.is_empty());
    *slot().write().expect("hippo_target lock poisoned") = normalized;
}

/// 当前生效的**显式**覆盖（没有就 `None`）。
pub fn base_url() -> Option<String> {
    slot().read().expect("hippo_target lock poisoned").clone()
}

/// 海马体 base URL 的**纯函数**部分 —— 环境变量那条路。
///
/// ⚠️ 保持纯函数（不吃全局状态）⇒ ⇒ 它可以被单独测，也可以被
/// [`resolve`] 复用 ⇒ ⇒ 「一条规则」真的只有一份。
pub fn resolve_hippocampus_url(
    telencephalon_public_url: Option<&str>,
    hippocampus_host: Option<&str>,
) -> String {
    let telencephalon_base = telencephalon_public_url.unwrap_or("http://localhost:60080");
    hippocampus_host
        .map(String::from)
        .unwrap_or_else(|| format!("{telencephalon_base}/hippocampus"))
}

/// 海马体的 base URL —— 完整优先级链。
pub fn resolve() -> String {
    if let Some(explicit) = base_url() {
        return explicit;
    }
    if let Ok(host) = std::env::var("HIPPOCAMPUS_HOST") {
        let host = host.trim();
        if !host.is_empty() {
            return normalize(host);
        }
    }
    let public = std::env::var("TELENCEPHALON_PUBLIC_URL")
        .ok()
        .or_else(|| std::env::var("CORTEX_PUBLIC_URL").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:60080".to_string());
    resolve_hippocampus_url(Some(&public), None)
}

/// 去掉尾斜杠 —— base URL 与具体路径要拼得起来。
fn normalize(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ 显式覆盖是**全局**状态 ⇒ ⇒ 用例必须**串行**，否则互相串台。
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn an_explicit_override_beats_everything() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_base_url(Some("http://remote-brain:60081/_zt/main/hippocampus/".to_string()));
        assert_eq!(resolve(), "http://remote-brain:60081/_zt/main/hippocampus");
        set_base_url(None);
    }

    #[test]
    fn clearing_the_override_goes_back_to_the_environment() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_base_url(Some("http://x:1/".to_string()));
        set_base_url(None);
        assert!(base_url().is_none());
        // ⚠️ 不断言具体值 —— 机器上有没有 `HIPPOCAMPUS_HOST` 取决于环境
        // ⇒ ⇒ 只断言「显式覆盖确实不再生效」。
        assert_ne!(resolve(), "http://x:1");
    }

    #[test]
    fn an_empty_override_is_treated_as_none() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_base_url(Some("   ".to_string()));
        assert!(base_url().is_none());
        set_base_url(None);
    }

    #[test]
    fn the_pure_function_keeps_its_shape() {
        assert_eq!(
            resolve_hippocampus_url(Some("http://myhost:60080"), None),
            "http://myhost:60080/hippocampus"
        );
        assert_eq!(
            resolve_hippocampus_url(None, Some("http://custom:9999")),
            "http://custom:9999"
        );
        assert_eq!(
            resolve_hippocampus_url(None, None),
            "http://localhost:60080/hippocampus"
        );
    }
}
