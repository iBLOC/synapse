use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::RwLock;

use super::builder::ProviderBuilder;
use super::encrypted_store::EncryptedFileStore;
use super::schema::ProviderSchema;
use super::schemas::all_schemas;
use super::store::{ConfigError, ConfigStore, InMemoryStore, ProviderConfig};
use crate::providers::{LLMError, LLMProvider, ProviderRegistry};

pub struct ProviderManager {
    store: Box<dyn ConfigStore>,
    schemas: HashMap<String, ProviderSchema>,
    providers: RwLock<HashMap<String, Arc<dyn LLMProvider>>>,
}

impl ProviderManager {
    pub fn builder() -> ProviderManagerBuilder {
        ProviderManagerBuilder::new()
    }

    pub fn schema(&self, provider_id: &str) -> Option<&ProviderSchema> {
        self.schemas.get(provider_id)
    }

    pub fn schemas(&self) -> Vec<&ProviderSchema> {
        self.schemas.values().collect()
    }

    pub fn get_config(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        self.store.get(provider_id)
    }

    pub fn get_masked(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        Ok(self.store.get(provider_id)?.map(|c| c.masked()))
    }

    pub fn save(&self, provider_id: &str, config: &ProviderConfig) -> Result<(), ConfigError> {
        self.store.set(provider_id, config)?;
        self.providers.write().remove(provider_id);
        Ok(())
    }

    pub fn delete(&self, provider_id: &str) -> Result<(), ConfigError> {
        self.store.delete(provider_id)?;
        self.providers.write().remove(provider_id);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, ConfigError> {
        self.store.list()
    }

    pub fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.read().get(provider_id).cloned()
    }

    pub fn get_or_create(&self, provider_id: &str) -> Result<Arc<dyn LLMProvider>, LLMError> {
        if let Some(provider) = self.get_provider(provider_id) {
            return Ok(provider);
        }

        let config = self.store.get(provider_id)
            .map_err(|e| LLMError::Api(e.to_string()))?
            .ok_or_else(|| LLMError::Api(format!("provider not configured: {provider_id}")))?;

        let schema = self.schemas.get(provider_id)
            .ok_or_else(|| LLMError::Api(format!("unknown provider: {provider_id}")))?;

        let provider = ProviderBuilder::new(schema.clone())
            .with_config(config)
            .build()?;

        self.providers.write().insert(provider_id.to_string(), provider.clone());
        Ok(provider)
    }

    pub fn get_or_prompt(&self, provider_id: &str) -> Result<Arc<dyn LLMProvider>, LLMError> {
        self.get_or_create(provider_id)
    }

    pub fn to_registry(&self) -> Result<ProviderRegistry, LLMError> {
        let mut registry = ProviderRegistry::new();

        for provider_id in self.store.list().map_err(|e| LLMError::Api(e.to_string()))? {
            if let Ok(provider) = self.get_or_create(&provider_id) {
                registry.register(&provider_id, provider);
            }
        }

        Ok(registry)
    }
}

pub struct ProviderManagerBuilder {
    store: Option<Box<dyn ConfigStore>>,
    schemas: HashMap<String, ProviderSchema>,
}

impl ProviderManagerBuilder {
    pub fn new() -> Self {
        Self {
            store: None,
            schemas: HashMap::new(),
        }
    }

    pub fn with_store(mut self, store: impl ConfigStore + 'static) -> Self {
        self.store = Some(Box::new(store));
        self
    }

    pub fn with_data_dir(mut self, dir: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let store = EncryptedFileStore::new(dir, &self.generate_default_password())?;
        self.store = Some(Box::new(store));
        Ok(self)
    }

    pub fn with_password(mut self, dir: impl AsRef<Path>, password: &str) -> Result<Self, ConfigError> {
        let store = EncryptedFileStore::new(dir, password)?;
        self.store = Some(Box::new(store));
        Ok(self)
    }

    pub fn with_schema(mut self, schema: ProviderSchema) -> Self {
        self.schemas.insert(schema.id.clone(), schema);
        self
    }

    pub fn with_all_schemas(mut self) -> Self {
        for schema in all_schemas() {
            self.schemas.insert(schema.id.clone(), schema);
        }
        self
    }

    pub fn build(self) -> Result<ProviderManager, ConfigError> {
        let store = self.store.unwrap_or_else(|| Box::new(InMemoryStore::new()));

        let schemas = if self.schemas.is_empty() {
            all_schemas().into_iter().map(|s| (s.id.clone(), s)).collect()
        } else {
            self.schemas
        };

        Ok(ProviderManager {
            store,
            schemas,
            providers: RwLock::new(HashMap::new()),
        })
    }

    fn generate_default_password(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"synapse-default-key");
        hex::encode(hasher.finalize())
    }
}

impl Default for ProviderManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_manager_in_memory() {
        let manager = ProviderManager::builder()
            .with_all_schemas()
            .build()
            .unwrap();

        assert!(manager.schema("openai").is_some());
        assert!(manager.schema("anthropic").is_some());
        assert!(manager.schema("unknown").is_none());
    }

    #[test]
    fn test_provider_manager_save_and_get() {
        let manager = ProviderManager::builder()
            .with_all_schemas()
            .build()
            .unwrap();

        let config = ProviderConfig::new("openai")
            .with_api_key("sk-test")
            .with_model("gpt-4o");

        manager.save("openai", &config).unwrap();

        let loaded = manager.get_config("openai").unwrap().unwrap();
        assert_eq!(loaded.api_key, Some("sk-test".to_string()));

        let masked = manager.get_masked("openai").unwrap().unwrap();
        assert_eq!(masked.api_key, Some("••••••••".to_string()));
    }

    #[test]
    fn test_provider_manager_encrypted() {
        let dir = tempfile::tempdir().unwrap();

        let manager = ProviderManager::builder()
            .with_password(dir.path(), "test-password")
            .unwrap()
            .with_all_schemas()
            .build()
            .unwrap();

        let config = ProviderConfig::new("anthropic")
            .with_api_key("sk-ant-secret")
            .with_model("claude-3-5-sonnet-20241022");

        manager.save("anthropic", &config).unwrap();

        let loaded = manager.get_config("anthropic").unwrap().unwrap();
        assert_eq!(loaded.api_key, Some("sk-ant-secret".to_string()));
    }
}
