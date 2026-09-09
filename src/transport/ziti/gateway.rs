//! ZitiGateway — the entry point for OpenZiti operations.
//!
//! Loads an identity JWT, exposes `dial` (client side) and `listen`
//! (dark-service side). One `ZitiGateway` per process is typical.

use tracing::info;
use ziti_sdk::{Context, ZitiStream};

use super::listener::ZitiListener;

pub struct ZitiGateway {
    context: Context,
}

impl ZitiGateway {
    pub async fn new(identity_path: &str) -> anyhow::Result<Self> {
        info!("Loading Ziti identity from: {}", identity_path);
        let context = Context::from_file(identity_path).await?;
        info!("Ziti gateway initialized successfully");
        Ok(Self { context })
    }

    pub async fn dial(&self, service_name: &str) -> anyhow::Result<ZitiStream> {
        info!("Dialing Ziti service: {}", service_name);
        let stream = self.context.dial(service_name).await?;
        info!("Connected to Ziti service: {}", service_name);
        Ok(stream)
    }

    pub async fn listen(&self, service_name: &str) -> anyhow::Result<ZitiListener> {
        info!("Starting Dark Service listener for: {}", service_name);
        let listener = self.context.listen(service_name).await?;
        info!("Dark Service registered: {}", service_name);
        Ok(ZitiListener::new(listener))
    }

    pub fn context(&self) -> &Context {
        &self.context
    }
}