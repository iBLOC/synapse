//! Configuration types for Ziti dark-service deployment.

use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::watch;
use tracing::{error, info};

#[derive(Debug, Clone, Deserialize)]
pub struct ZitiConfig {
    pub identity_path: String,
    pub service_name: String,
    pub controller_url: Option<String>,
    pub log_level: Option<String>,
}

impl ZitiConfig {
    pub fn from_env() -> Result<Self> {
        let identity_path =
            std::env::var("ZITI_IDENTITY").unwrap_or_else(|_| "identity.json".to_string());
        let service_name =
            std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "default.ziti".to_string());

        Ok(Self {
            identity_path,
            service_name,
            controller_url: None,
            log_level: None,
        })
    }
}

/// Watches a JSON config file on disk and emits updated configs via a
/// tokio watch channel. Polls mtime every 5 seconds — fine for
/// identity/service-name rotations, not fine for sub-second reloads.
pub struct ConfigWatcher {
    config_path: PathBuf,
    tx: watch::Sender<ZitiConfig>,
    rx: watch::Receiver<ZitiConfig>,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let config = Self::load_config(&config_path)?;
        let (tx, rx) = watch::channel(config);

        Ok(Self {
            config_path,
            tx,
            rx,
        })
    }

    fn load_config(path: &PathBuf) -> Result<ZitiConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: ZitiConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn subscribe(&self) -> watch::Receiver<ZitiConfig> {
        self.rx.clone()
    }

    pub async fn watch(&self) {
        let config_path = self.config_path.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut last_modified = None;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                match std::fs::metadata(&config_path) {
                    Ok(metadata) => {
                        let modified = metadata.modified().ok();

                        if modified != last_modified {
                            last_modified = modified;

                            match Self::load_config(&config_path) {
                                Ok(config) => {
                                    info!("Config file updated, reloading");
                                    if let Err(e) = tx.send(config) {
                                        error!("Failed to send config update: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to reload config: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to check config file: {}", e);
                    }
                }
            }
        });
    }

    pub fn get(&self) -> ZitiConfig {
        self.rx.borrow().clone()
    }
}