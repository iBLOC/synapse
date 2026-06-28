use crate::channels::message::{ChannelId, ChannelMessage, PeerInfo, PeerKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteBinding {
    pub match_criteria: RouteMatch,
    pub agent_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteMatch {
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_kind: Option<PeerKind>,
    pub peer_id: Option<String>,
    pub thread_id: Option<String>,
    pub guild_id: Option<String>,
    pub team_id: Option<String>,
}

impl RouteMatch {
    pub fn channel(id: impl Into<String>) -> Self {
        Self {
            channel: Some(id.into()),
            ..Default::default()
        }
    }

    pub fn account(channel: impl Into<String>, account_id: impl Into<String>) -> Self {
        Self {
            channel: Some(channel.into()),
            account_id: Some(account_id.into()),
            ..Default::default()
        }
    }

    pub fn peer(channel: impl Into<String>, peer: PeerInfo) -> Self {
        Self {
            channel: Some(channel.into()),
            peer_kind: Some(peer.kind),
            peer_id: Some(peer.id),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterConfig {
    pub default_agent: Option<String>,
    pub bindings: Vec<RouteBinding>,
}

impl RouterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default(mut self, agent_id: impl Into<String>) -> Self {
        self.default_agent = Some(agent_id.into());
        self
    }

    pub fn add_binding(mut self, binding: RouteBinding) -> Self {
        self.bindings.push(binding);
        self
    }
}

#[derive(Clone)]
pub struct Router {
    config: std::sync::Arc<parking_lot::RwLock<RouterConfig>>,
}

impl Router {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config: std::sync::Arc::new(parking_lot::RwLock::new(config)),
        }
    }

    pub fn default_router() -> Self {
        Self::new(RouterConfig::default())
    }

    pub fn route(&self, message: &ChannelMessage) -> Option<String> {
        let config = self.config.read();

        for binding in &config.bindings {
            if Self::matches(&binding.match_criteria, message) {
                return Some(binding.agent_id.clone());
            }
        }

        config.default_agent.clone()
    }

    fn matches(criteria: &RouteMatch, message: &ChannelMessage) -> bool {
        if let Some(ref ch) = criteria.channel {
            if message.channel_id.as_str() != ch {
                return false;
            }
        }

        if let Some(ref acc) = criteria.account_id {
            match &message.account_id {
                Some(msg_acc) if msg_acc == acc => {}
                _ => return false,
            }
        }

        if let Some(kind) = criteria.peer_kind {
            if message.to.peer.kind != kind {
                return false;
            }
        }

        if let Some(ref pid) = criteria.peer_id {
            if message.to.peer.id != *pid {
                return false;
            }
        }

        if let Some(ref tid) = criteria.thread_id {
            match &message.thread_root {
                Some(root) if root.to_string() == *tid => {}
                _ => return false,
            }
        }

        true
    }

    pub fn add_binding(&self, binding: RouteBinding) {
        self.config.write().bindings.push(binding);
    }

    pub fn set_default(&self, agent_id: impl Into<String>) {
        self.config.write().default_agent = Some(agent_id.into());
    }

    pub fn bindings(&self) -> Vec<RouteBinding> {
        self.config.read().bindings.clone()
    }

    pub fn default_agent(&self) -> Option<String> {
        self.config.read().default_agent.clone()
    }
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.config.read();
        f.debug_struct("Router")
            .field("default_agent", &config.default_agent)
            .field("bindings_count", &config.bindings.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(channel: &str, peer_id: &str, kind: PeerKind) -> ChannelMessage {
        ChannelMessage::new(
            channel,
            PeerInfo::dm("sender"),
            crate::channels::message::ChannelTarget::new(channel, match kind {
                PeerKind::Dm => PeerInfo::dm(peer_id),
                PeerKind::Group => PeerInfo::group(peer_id),
                PeerKind::Channel => PeerInfo::channel(peer_id),
                PeerKind::Thread => PeerInfo::dm(peer_id),
            }),
            "test",
        )
    }

    #[test]
    fn test_default_router_no_match() {
        let router = Router::default_router();
        let msg = make_message("telegram", "user1", PeerKind::Dm);
        assert!(router.route(&msg).is_none());
    }

    #[test]
    fn test_router_with_default() {
        let router = Router::new(RouterConfig::new().with_default("main"));
        let msg = make_message("telegram", "user1", PeerKind::Dm);
        assert_eq!(router.route(&msg), Some("main".to_string()));
    }

    #[test]
    fn test_router_channel_match() {
        let router = Router::new(
            RouterConfig::new()
                .with_default("main")
                .add_binding(RouteBinding {
                    match_criteria: RouteMatch::channel("slack"),
                    agent_id: "slack_agent".to_string(),
                }),
        );

        let slack_msg = make_message("slack", "user1", PeerKind::Dm);
        assert_eq!(router.route(&slack_msg), Some("slack_agent".to_string()));

        let tg_msg = make_message("telegram", "user1", PeerKind::Dm);
        assert_eq!(router.route(&tg_msg), Some("main".to_string()));
    }

    #[test]
    fn test_router_peer_match() {
        let router = Router::new(
            RouterConfig::new()
                .add_binding(RouteBinding {
                    match_criteria: RouteMatch::peer("telegram", PeerInfo::dm("admin")),
                    agent_id: "admin_agent".to_string(),
                }),
        );

        let admin_msg = make_message("telegram", "admin", PeerKind::Dm);
        assert_eq!(router.route(&admin_msg), Some("admin_agent".to_string()));

        let user_msg = make_message("telegram", "user1", PeerKind::Dm);
        assert!(router.route(&user_msg).is_none());
    }
}