use crate::schema::{ToolArgument, ToolInfo, ToolResult};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type ToolResult_ = Result<String, ToolError>;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("execution error: {0}")]
    Execution(String),
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("timeout")]
    Timeout,
}

pub trait BaseTool: Send + Sync {
    fn info(&self) -> impl std::future::Future<Output = Result<ToolInfo, ToolError>> + Send;
}

#[async_trait]
pub trait InvokableTool: BaseTool + Send + Sync {
    async fn invoke(&self, arguments: String) -> ToolResult_;
}

#[async_trait]
pub trait StreamableTool: BaseTool + Send + Sync {
    async fn stream(&self, arguments: String) -> Result<StringStreamReader, ToolError>;
}

#[async_trait]
pub trait EnhancedInvokableTool: BaseTool + Send + Sync {
    async fn invoke_enhanced(&self, arguments: ToolArgument) -> Result<ToolResult, ToolError>;
}

#[async_trait]
pub trait EnhancedStreamableTool: BaseTool + Send + Sync {
    async fn stream_enhanced(
        &self,
        arguments: ToolArgument,
    ) -> Result<ToolResultStreamReader, ToolError>;
}

pub struct StringStreamReader {
    receiver: tokio::sync::mpsc::Receiver<Result<String, ToolError>>,
}

impl StringStreamReader {
    pub fn new(channel_size: usize) -> (StringStreamWriter, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel(channel_size);
        let writer = StringStreamWriter { tx };
        let reader = StringStreamReader { receiver: rx };
        (writer, reader)
    }

    pub async fn next(&mut self) -> Option<Result<String, ToolError>> {
        self.receiver.recv().await
    }
}

impl Unpin for StringStreamReader {}

impl Stream for StringStreamReader {
    type Item = Result<String, ToolError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(item) => Poll::Ready(item),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct StringStreamWriter {
    tx: tokio::sync::mpsc::Sender<Result<String, ToolError>>,
}

impl StringStreamWriter {
    pub async fn send(&self, item: Result<String, ToolError>) -> bool {
        self.tx.send(item).await.is_ok()
    }

    pub async fn send_value(&self, value: String) -> bool {
        self.send(Ok(value)).await
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

impl Clone for StringStreamWriter {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

pub type StringBoxStreamReader = Pin<Box<dyn Stream<Item = Result<String, ToolError>> + Send>>;

pub struct ToolResultStreamReader {
    receiver: tokio::sync::mpsc::Receiver<Result<ToolResult, ToolError>>,
}

impl ToolResultStreamReader {
    pub fn new(channel_size: usize) -> (ToolResultStreamWriter, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel(channel_size);
        let writer = ToolResultStreamWriter { tx };
        let reader = ToolResultStreamReader { receiver: rx };
        (writer, reader)
    }

    pub async fn next(&mut self) -> Option<Result<ToolResult, ToolError>> {
        self.receiver.recv().await
    }
}

impl Unpin for ToolResultStreamReader {}

impl Stream for ToolResultStreamReader {
    type Item = Result<ToolResult, ToolError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(item) => Poll::Ready(item),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct ToolResultStreamWriter {
    tx: tokio::sync::mpsc::Sender<Result<ToolResult, ToolError>>,
}

impl ToolResultStreamWriter {
    pub async fn send(&self, item: Result<ToolResult, ToolError>) -> bool {
        self.tx.send(item).await.is_ok()
    }

    pub async fn send_value(&self, value: ToolResult) -> bool {
        self.send(Ok(value)).await
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

impl Clone for ToolResultStreamWriter {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}
