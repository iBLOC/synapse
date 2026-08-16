//! Channel Adapter system - 参考 OpenClaw 设计
//!
//! 实现 Channel Adapter Pattern，将不同的消息平台(WhatsApp, Telegram,
//! Discord, Slack, 等)统一为内部接口。

pub mod message;
pub mod plugin;
pub mod dock;
pub mod registry;
pub mod router;
pub mod gateway;

pub use message::{ChannelMessage, ChannelTarget, ChannelResponse, MessageId, ChannelId, PeerInfo, PeerKind};
pub use plugin::{
    ChannelPlugin, ChannelMeta, ChannelCapabilities,
    ConfigAdapter, GatewayAdapter, OutboundAdapter, StatusAdapter,
    ChannelError, ChannelResult, ChannelStatus,
};
pub use dock::ChannelDock;
pub use registry::ChannelRegistry;
pub use router::{Router, RouterConfig, RouteBinding, RouteMatch};
pub use gateway::{Gateway, MessageHandler, MessageHandlerFuture};