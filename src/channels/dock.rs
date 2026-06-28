use serde::{Deserialize, Serialize};

use super::plugin::{ChannelCapabilities, ChannelMeta, ChannelStatus, ChannelStatus as _};
use std::any::Any;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDock {
    pub id: String,
    pub meta: ChannelMeta,
    pub capabilities: ChannelCapabilities,
    pub outbound_adapter: bool,
    pub status_adapter: bool,
}

impl ChannelDock {
    pub fn from_meta(meta: ChannelMeta, capabilities: ChannelCapabilities) -> Self {
        Self {
            id: meta.id.clone(),
            meta,
            capabilities,
            outbound_adapter: true,
            status_adapter: true,
        }
    }

    pub fn from_plugin<P: super::plugin::ChannelPlugin + ?Sized>(plugin: &P) -> Self {
        Self {
            id: plugin.id().to_string(),
            meta: plugin.meta().clone(),
            capabilities: plugin.capabilities().clone(),
            outbound_adapter: plugin.outbound().is_some(),
            status_adapter: plugin.status().is_some(),
        }
    }

    pub fn supports_dm(&self) -> bool {
        self.capabilities.supports_dm
    }

    pub fn supports_group(&self) -> bool {
        self.capabilities.supports_group
    }

    pub fn supports_thread(&self) -> bool {
        self.capabilities.supports_thread
    }

    pub fn is_configured(&self) -> bool {
        true
    }
}

pub type ChannelHandle = Box<dyn Any + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_dock_from_meta() {
        let meta = ChannelMeta::new("test", "Test", "Test channel");
        let caps = ChannelCapabilities {
            supports_dm: true,
            supports_group: true,
            ..Default::default()
        };

        let dock = ChannelDock::from_meta(meta, caps);
        assert_eq!(dock.id, "test");
        assert!(dock.supports_dm());
        assert!(dock.supports_group());
        assert!(!dock.supports_thread());
    }
}