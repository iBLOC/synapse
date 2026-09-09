//! ZitiListener — accept incoming ZitiStreams on a dark service.
//!
//! `DarkServiceHandler` is a low-level escape hatch for callers that
//! want to handle each connection themselves (e.g. binary protocols).
//! For axum Routers, prefer the top-level `serve` convenience.

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use ziti_sdk::{ZitiListener as RawZitiListener, ZitiStream};

pub struct ZitiListener {
    inner: RawZitiListener,
}

impl ZitiListener {
    pub fn new(inner: RawZitiListener) -> Self {
        Self { inner }
    }

    pub async fn accept(&mut self) -> Result<ZitiStream> {
        let stream = self.inner.accept().await?;
        Ok(stream)
    }

    pub fn service_name(&self) -> &str {
        self.inner.service_name()
    }
}

pub trait DarkServiceHandler: Send + Sync + 'static {
    fn handle(
        &self,
        stream: ZitiStream,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

pub struct DarkServiceListener {
    gateway: Arc<super::gateway::ZitiGateway>,
    service_name: String,
}

impl DarkServiceListener {
    pub fn new(gateway: Arc<super::gateway::ZitiGateway>, service_name: String) -> Self {
        Self {
            gateway,
            service_name,
        }
    }

    pub async fn serve<H: DarkServiceHandler>(&self, handler: Arc<H>) -> Result<()> {
        let mut listener = self.gateway.listen(&self.service_name).await?;

        loop {
            let stream = listener.accept().await?;
            let handler = handler.clone();

            tokio::spawn(async move {
                if let Err(e) = handler.handle(stream).await {
                    tracing::error!("Error handling connection: {}", e);
                }
            });
        }
    }
}