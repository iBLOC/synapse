//! ZitiGateway — the entry point for OpenZiti operations.
//!
//! Loads an identity file, exposes `dial` (client side) and `listen`
//! (dark-service side). One `ZitiGateway` per process is typical.
//!
//! ## v2
//!
//! 底下是**自建的 v2 实现**（super::v2）—— 不再走第三方 ziti-sdk：
//! 那套说的是 OpenZiti **v1** 的协议（WebSocket + 自制 ZitiMessage），
//! 而现在部署的是 **v2.0.8**（channel 二进制帧 ＋ 60783+ 内容类型）。

use tracing::info;

use super::listener::ZitiListener;
use super::v2::context::Context;
use super::ZitiStream;

pub struct ZitiGateway {
    context: Context,
}

impl ZitiGateway {
    pub async fn new(identity_path: &str) -> anyhow::Result<Self> {
        info!("Loading Ziti identity from: {}", identity_path);
        let context = Context::from_file(identity_path)
            .await
            .map_err(|e| anyhow::anyhow!("加载 Ziti 身份失败: {e}"))?;
        info!("Ziti gateway initialized successfully");
        Ok(Self { context })
    }

    pub async fn dial(&self, service_name: &str) -> anyhow::Result<ZitiStream> {
        info!("Dialing Ziti service: {}", service_name);
        let stream = self
            .context
            .dial(service_name)
            .await
            .map_err(|e| anyhow::anyhow!("拨号 {service_name} 失败: {e}"))?;
        info!("Connected to Ziti service: {}", service_name);
        Ok(stream)
    }

    pub async fn listen(&self, service_name: &str) -> anyhow::Result<ZitiListener> {
        info!("Starting Dark Service listener for: {}", service_name);
        let listener = self
            .context
            .listen(service_name)
            .await
            .map_err(|e| anyhow::anyhow!("托管 {service_name} 失败: {e}"))?;
        info!("Dark Service registered: {}", service_name);
        Ok(ZitiListener::new(listener))
    }

    pub fn context(&self) -> &Context {
        &self.context
    }
}
