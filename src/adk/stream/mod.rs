//! Agent execution streaming — event bus, forwarding helpers, and the
//! default `AgentStream` type alias.

pub mod event;
pub mod forward;

pub use event::{AgentStream, AgentStreamEvent};
pub use forward::forward_to;
