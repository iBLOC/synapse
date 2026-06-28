use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::dock::ChannelDock;
use super::plugin::{ChannelPlugin, ChannelStatus, OutboundAdapter};

#[derive(Clone)]
pub struct ChannelRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn ChannelPlugin>>>>,
    docks: Arc<RwLock<HashMap<String, ChannelDock>>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            docks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register<P: ChannelPlugin + 'static>(&self, plugin: P) {
        let plugin = Arc::new(plugin);
        let dock = ChannelDock::from_plugin(&*plugin);
        let id = dock.id.clone();

        self.plugins.write().insert(id.clone(), plugin);
        self.docks.write().insert(id, dock);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn ChannelPlugin>> {
        self.plugins.read().get(id).cloned()
    }

    pub fn get_dock(&self, id: &str) -> Option<ChannelDock> {
        self.docks.read().get(id).cloned()
    }

    pub fn list_docks(&self) -> Vec<ChannelDock> {
        self.docks.read().values().cloned().collect()
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.plugins.read().keys().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }

    pub fn unregister(&self, id: &str) -> Option<Arc<dyn ChannelPlugin>> {
        self.docks.write().remove(id);
        self.plugins.write().remove(id)
    }

    pub async fn send(
        &self,
        channel_id: &str,
        account: &serde_json::Value,
        target: &super::message::ChannelTarget,
        response: &super::message::ChannelResponse,
    ) -> Result<(), super::plugin::ChannelError> {
        let plugin = self
            .get(channel_id)
            .ok_or_else(|| super::plugin::ChannelError::Unavailable(channel_id.to_string()))?;

        let outbound = plugin
            .outbound()
            .ok_or_else(|| super::plugin::ChannelError::Unavailable("No outbound adapter".to_string()))?;

        outbound.send(account, target, response).await
    }

    pub async fn status(
        &self,
        channel_id: &str,
        account: &serde_json::Value,
    ) -> Result<ChannelStatus, super::plugin::ChannelError> {
        let plugin = self
            .get(channel_id)
            .ok_or_else(|| super::plugin::ChannelError::Unavailable(channel_id.to_string()))?;

        let status = plugin
            .status()
            .ok_or_else(|| super::plugin::ChannelError::Unavailable("No status adapter".to_string()))?;

        status.status(account).await
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ChannelRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelRegistry")
            .field("count", &self.count())
            .field("ids", &self.list_ids())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::plugin::{ChannelCapabilities, ChannelMeta};

    struct MockChannel;

    #[async_trait::async_trait]
    impl super::super::plugin::ChannelPlugin for MockChannel {
        fn id(&self) -> &str {
            "mock"
        }
        fn meta(&self) -> &ChannelMeta {
            todo!()
        }
        fn capabilities(&self) -> &ChannelCapabilities {
            todo!()
        }
    }

    #[test]
    fn test_registry_empty() {
        let registry = ChannelRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_registry_register_get() {
        let registry = ChannelRegistry::new();
        let dock = ChannelDock::from_meta(
            ChannelMeta::new("test", "Test", "Test"),
            ChannelCapabilities::default(),
        );

        registry.plugins.write().insert(
            "test".to_string(),
            Arc::new(MockChannel),
        );
        registry.docks.write().insert("test".to_string(), dock);

        assert_eq!(registry.count(), 1);
        assert!(registry.get("test").is_some());
    }
}