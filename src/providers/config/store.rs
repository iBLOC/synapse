use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(String),
    #[error("encryption error: {0}")]
    Encryption(String),
    #[error("decryption error: {0}")]
    Decryption(String),
    #[error("provider not found: {0}")]
    NotFound(String),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ProviderConfig {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            api_key: None,
            base_url: None,
            model: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    pub fn masked(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            api_key: self.api_key.as_ref().map(|_| "••••••••".to_string()),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            extra: self.extra.clone(),
        }
    }
}

pub trait ConfigStore: Send + Sync {
    fn get(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError>;
    fn set(&self, provider_id: &str, config: &ProviderConfig) -> Result<(), ConfigError>;
    fn delete(&self, provider_id: &str) -> Result<(), ConfigError>;
    fn list(&self) -> Result<Vec<String>, ConfigError>;
}

pub struct InMemoryStore {
    data: RwLock<HashMap<String, ProviderConfig>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigStore for InMemoryStore {
    fn get(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        Ok(self.data.read().get(provider_id).cloned())
    }

    fn set(&self, provider_id: &str, config: &ProviderConfig) -> Result<(), ConfigError> {
        self.data.write().insert(provider_id.to_string(), config.clone());
        Ok(())
    }

    fn delete(&self, provider_id: &str) -> Result<(), ConfigError> {
        self.data.write().remove(provider_id);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, ConfigError> {
        Ok(self.data.read().keys().cloned().collect())
    }
}

pub struct FileStore {
    dir: PathBuf,
}

impl FileStore {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| ConfigError::Io(format!("create dir: {e}")))?;
        Ok(Self { dir })
    }

    fn config_path(&self, provider_id: &str) -> PathBuf {
        self.dir.join(format!("{provider_id}.json"))
    }
}

impl ConfigStore for FileStore {
    fn get(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        let path = self.config_path(provider_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::Io(format!("read: {e}")))?;
        let config: ProviderConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::Serialization(format!("parse: {e}")))?;
        Ok(Some(config))
    }

    fn set(&self, provider_id: &str, config: &ProviderConfig) -> Result<(), ConfigError> {
        let path = self.config_path(provider_id);
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| ConfigError::Serialization(format!("serialize: {e}")))?;
        std::fs::write(&path, content)
            .map_err(|e| ConfigError::Io(format!("write: {e}")))?;
        Ok(())
    }

    fn delete(&self, provider_id: &str) -> Result<(), ConfigError> {
        let path = self.config_path(provider_id);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| ConfigError::Io(format!("delete: {e}")))?;
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, ConfigError> {
        let mut providers = Vec::new();
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| ConfigError::Io(format!("read dir: {e}")))?;
        for entry in entries {
            let entry = entry.map_err(|e| ConfigError::Io(format!("read entry: {e}")))?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    providers.push(stem.to_string_lossy().to_string());
                }
            }
        }
        Ok(providers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_store() {
        let store = InMemoryStore::new();
        let config = ProviderConfig::new("openai")
            .with_api_key("sk-test")
            .with_model("gpt-4o");

        store.set("openai", &config).unwrap();

        let loaded = store.get("openai").unwrap().unwrap();
        assert_eq!(loaded.provider, "openai");
        assert_eq!(loaded.api_key, Some("sk-test".to_string()));
        assert_eq!(loaded.model, Some("gpt-4o".to_string()));

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed.contains(&"openai".to_string()));

        store.delete("openai").unwrap();
        assert!(store.get("openai").unwrap().is_none());
    }

    #[test]
    fn test_file_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileStore::new(dir.path()).unwrap();

        let config = ProviderConfig::new("anthropic")
            .with_api_key("sk-ant-test")
            .with_model("claude-3-5-sonnet-20241022");

        store.set("anthropic", &config).unwrap();

        let loaded = store.get("anthropic").unwrap().unwrap();
        assert_eq!(loaded.provider, "anthropic");
        assert_eq!(loaded.api_key, Some("sk-ant-test".to_string()));

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);

        store.delete("anthropic").unwrap();
        assert!(store.get("anthropic").unwrap().is_none());
    }

    #[test]
    fn test_masked_config() {
        let config = ProviderConfig::new("openai")
            .with_api_key("sk-secret-key")
            .with_model("gpt-4o");

        let masked = config.masked();
        assert_eq!(masked.api_key, Some("••••••••".to_string()));
        assert_eq!(masked.model, Some("gpt-4o".to_string()));
    }
}
