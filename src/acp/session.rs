//! Public re-exports + the internal prompt-request envelope.
//!
//! [`AcpClient::session`] returns an [`AcpSession`] — a cheap borrowed
//! handle for issuing [`AcpSession::prompt`]s.

use tokio::sync::mpsc;

use crate::adk::stream::AgentStreamEvent;

/// One prompt request routed from the public API to the background
/// dispatch task.
pub(crate) struct PromptRequest {
    pub text: String,
    /// Channel back to the caller; receiving `None` from this means
    /// "the session is done, close the consumer-side stream".
    pub reply: mpsc::Sender<Option<AgentStreamEvent>>,
}

/// Borrowed handle returned by [`super::AcpClient::session`].
pub use super::client::AcpSessionHandle as AcpSession;