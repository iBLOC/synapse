use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub type ModelResult<T> = Result<T, ModelError>;

#[derive(Debug, thiserror::Error, Clone)]
pub enum ModelError {
    #[error("api error: {0}")]
    Api(String),
    #[error("rate limit: {0}")]
    RateLimit(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("context length exceeded")]
    ContextLengthExceeded,
    #[error("timeout")]
    Timeout,
    #[error("stream error: {0}")]
    StreamError(String),
}

#[async_trait]
pub trait BaseChatModel: Send + Sync {
    async fn generate(
        &self,
        input: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> ModelResult<crate::schema::Message>;

    async fn stream(
        &self,
        input: Vec<crate::schema::Message>,
        options: GenerateOptions,
    ) -> ModelResult<StreamReader>;
}

pub struct GenerateOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<crate::schema::ToolInfo>>,
    pub tool_choice: Option<ToolChoice>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            model: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    None,
    Function { name: String },
}

pub struct StreamReader {
    receiver: mpsc::Receiver<Result<crate::schema::Message, ModelError>>,
}

impl StreamReader {
    pub fn new(channel_size: usize) -> (StreamWriter, Self) {
        let (tx, rx) = mpsc::channel(channel_size);
        let writer = StreamWriter { tx };
        let reader = StreamReader { receiver: rx };
        (writer, reader)
    }

    pub fn from_receiver(
        receiver: mpsc::Receiver<Result<crate::schema::Message, ModelError>>,
    ) -> Self {
        Self { receiver }
    }

    pub async fn next(&mut self) -> Option<Result<crate::schema::Message, ModelError>> {
        self.receiver.recv().await
    }
}

impl Unpin for StreamReader {}

impl Stream for StreamReader {
    type Item = Result<crate::schema::Message, ModelError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(item) => Poll::Ready(item),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct StreamWriter {
    tx: mpsc::Sender<Result<crate::schema::Message, ModelError>>,
}

impl StreamWriter {
    pub async fn send(&self, item: Result<crate::schema::Message, ModelError>) -> bool {
        self.tx.send(item).await.is_ok()
    }

    pub async fn send_value(&self, value: crate::schema::Message) -> bool {
        self.send(Ok(value)).await
    }

    pub async fn send_error(&self, error: ModelError) -> bool {
        self.send(Err(error)).await
    }

    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

impl Clone for StreamWriter {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

pub type BoxStreamReader =
    Pin<Box<dyn Stream<Item = Result<crate::schema::Message, ModelError>> + Send>>;

pub mod openai;
pub use openai::OpenAIModel;
#[cfg(not(target_arch = "wasm32"))]
pub use openai::{OpenAIChatModel, OpenAIChatModelBuilder, OpenAIConfig};
