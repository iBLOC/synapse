//! `LobeApp` convenience builder: build a lobe binary in 10 lines.
//!
//! ```ignore
//! use synapse::lobe::*;
//!
//! let app = LobeApp::builder()
//!     .name("my_agent")
//!     .bind("127.0.0.1:8091")
//!     .skill("my_skill", Arc::new(MySkill::new()))
//!     .build();
//! app.run().await?;
//! ```

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use crate::a2a::card::AgentCard;
use crate::lobe::{build_a2a_router, register_with_registry, StatelessSkillRouter};

/// ⚠️⚠️ **默认监听地址 —— 必须是回环**。
///
/// ⇒ ⇒ **宽绑定（`0.0.0.0`）必须显式给**（环境变量 `LOBE_BIND` 或
/// ⇒ ⇒ ⇒ `.bind(...)`）⇒ ⇒ 理由有**两条**，少一条都不够：
/// ⇒ ⇒ **① 暴露面**：默认安装不该在**局域网**上可读（同一台机器上的别的进程
/// ⇒ ⇒ ⇒ 就能打你的叶区）⇒ ⇒ **暴露面必须显式**。
/// ⇒ ⇒ **② 形状**：`build()` 把 **card 的 URL 直接从监听地址拼出来**
/// ⇒ ⇒ ⇒ ⇒ 默认宽绑定 ⇒ ⇒ card 会报出 `http://0.0.0.0:8090`
/// ⇒ ⇒ ⇒ ⇒ 而 **`0.0.0.0` 不是一个可连接的地址**（它是「监听所有网卡」）
/// ⇒ ⇒ ⇒ ⇒ ⇒ 单机上碰巧能通、换台机器**永远连不上**
/// ⇒ ⇒ ⇒ ⇒ ⇒ 「注册成功、永远连不上」—— 又一个「失败长得像成功」。
pub const DEFAULT_LOBE_BIND: &str = "127.0.0.1:8090";

pub struct LobeApp {
    pub name: String,
    pub bind: SocketAddr,
    pub card: AgentCard,
    pub skills: Arc<StatelessSkillRouter>,
}

pub struct LobeAppBuilder {
    name: String,
    bind: String,
    description: String,
    skills: StatelessSkillRouter,
}

impl LobeAppBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bind: std::env::var("LOBE_BIND").unwrap_or_else(|_| DEFAULT_LOBE_BIND.into()),
            description: "".into(),
            skills: StatelessSkillRouter::new(),
        }
    }

    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.bind = addr.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }

    pub fn skill(mut self, skill: Arc<dyn crate::lobe::StatelessSkill>) -> Self {
        self.skills.register(skill);
        self
    }

    pub fn build(self) -> LobeApp {
        let bind: SocketAddr = self.bind.parse().expect("invalid bind addr");
        let url = format!("http://{bind}");
        let mut card = AgentCard::new(
            &self.name,
            &self.description,
            &url,
            env!("CARGO_PKG_VERSION"),
        );
        for s in self.skills.list_skills() {
            card = card.skills(vec![crate::a2a::card::SkillReference::new(s, s)]);
        }
        LobeApp { name: self.name, bind, card, skills: Arc::new(self.skills) }
    }
}

impl LobeApp {
    pub fn builder() -> LobeAppBuilder { LobeAppBuilder::new("lobe") }

    pub fn name(&self) -> &str { &self.name }
    pub fn bind(&self) -> SocketAddr { self.bind }
    pub fn card(&self) -> &AgentCard { &self.card }

    pub fn router(&self) -> Router {
        build_a2a_router(self.card.clone(), Arc::clone(&self.skills))
    }

    /// Run: register with service registry, serve axum, wait for Ctrl-C.
    pub async fn run(self) -> Result<(), String> {
        if let Ok(registry_url) = std::env::var("REGISTRY_URL") {
            if let Err(e) = register_with_registry(&registry_url, &self.card).await {
                tracing::warn!("service registration failed: {e}");
            } else {
                tracing::info!("registered with service registry at {registry_url}");
            }
        }

        let listener = TcpListener::bind(self.bind).await
            .map_err(|e| format!("bind: {e}"))?;
        tracing::info!("{} listening on {}", self.name, self.bind);
        let app = self.router();
        axum::serve(listener, app).await
            .map_err(|e| format!("serve: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️⚠️ **默认监听必须是回环** —— 宽绑定必须显式（`LOBE_BIND` / `.bind(...)`）。
    ///
    /// ⇒ ⇒ 这条钉的是**契约**，不是实现细节 ⇒ ⇒ 谁把默认值悄悄改回
    /// ⇒ ⇒ `0.0.0.0`，这条当场红 ⇒ ⇒ 而不是等某台机器上「局域网可读」。
    #[test]
    fn the_default_bind_is_loopback() {
        let addr: SocketAddr = DEFAULT_LOBE_BIND.parse().expect("默认地址必须可解析");
        assert!(
            addr.ip().is_loopback(),
            "默认监听必须回环（{DEFAULT_LOBE_BIND}）⇒ 宽绑定必须显式"
        );
    }
}
