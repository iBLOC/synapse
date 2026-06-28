use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

use crate::channels::message::{ChannelMessage, ChannelResponse};
use crate::channels::registry::ChannelRegistry;
use crate::channels::router::Router;

pub type MessageHandlerFuture = Pin<Box<dyn Future<Output = crate::channels::plugin::ChannelResult<ChannelResponse>> + Send>>;
pub type MessageHandler = Arc<dyn Fn(ChannelMessage) -> MessageHandlerFuture + Send + Sync>;

pub struct Gateway {
    pub channels: ChannelRegistry,
    pub router: Router,
    pub message_handler: parking_lot::RwLock<Option<MessageHandler>>,
}

impl Gateway {
    pub fn new(channels: ChannelRegistry, router: Router) -> Self {
        Self {
            channels,
            router,
            message_handler: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_message_handler<F>(&self, handler: F)
    where
        F: Fn(ChannelMessage) -> MessageHandlerFuture + Send + Sync + 'static,
    {
        *self.message_handler.write() = Some(Arc::new(handler));
    }

    pub async fn handle_message(&self, message: ChannelMessage) -> crate::channels::plugin::ChannelResult<ChannelResponse> {
        let agent_id = self.router.route(&message);
        tracing::info!(channel = %message.channel_id, agent = ?agent_id, "Routing message");

        let handler = self.message_handler.read().clone();
        match handler {
            Some(h) => h(message).await,
            None => Err(crate::channels::plugin::ChannelError::Internal(
                "No message handler registered".to_string(),
            )),
        }
    }
}

impl std::fmt::Debug for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gateway")
            .field("channels", &self.channels)
            .field("router", &self.router)
            .field("has_handler", &self.message_handler.read().is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::message::PeerInfo;
    use crate::channels::plugin::ChannelError;

    #[tokio::test]
    async fn test_gateway_no_handler() {
        let channels = ChannelRegistry::new();
        let router = Router::default_router();
        let gateway = Gateway::new(channels, router);

        let msg = ChannelMessage::new(
            "test",
            PeerInfo::dm("user"),
            crate::channels::message::ChannelTarget::new("test", PeerInfo::dm("user")),
            "hello",
        );

        let result = gateway.handle_message(msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gateway_with_handler() {
        let channels = ChannelRegistry::new();
        let router = Router::default_router();
        let gateway = Gateway::new(channels, router);

        gateway.set_message_handler(|msg| {
            Box::pin(async move {
                Ok(ChannelResponse::new(format!("Got: {}", msg.text)))
            })
        });

        let msg = ChannelMessage::new(
            "test",
            PeerInfo::dm("user"),
            crate::channels::message::ChannelTarget::new("test", PeerInfo::dm("user")),
            "hello",
        );

        let result = gateway.handle_message(msg).await.unwrap();
        assert_eq!(result.text, "Got: hello");
    }
}