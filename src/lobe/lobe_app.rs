//! `LobeApp` convenience builder: build a lobe binary in 10 lines.
//!
//! ```ignore
//! use synapse::lobe::*;
//!
//! let app = LobeApp::builder()
//!     .name("my_agent")
//!     .bind("0.0.0.0:8091")
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
            bind: std::env::var("LOBE_BIND").unwrap_or_else(|_| "0.0.0.0:8090".into()),
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
