use std::collections::HashMap;
use std::sync::Arc;

use super::{LLMError, LLMProvider, LLMResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    default_provider: Option<String>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn LLMProvider>) {
        let name = name.into();
        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }
        self.providers.insert(name, provider);
    }

    pub fn set_default(&mut self, name: impl Into<String>) {
        self.default_provider = Some(name.into());
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn LLMProvider>> {
        self.providers.get(name)
    }

    pub fn get_default(&self) -> Option<&Arc<dyn LLMProvider>> {
        self.default_provider.as_ref().and_then(|n| self.providers.get(n))
    }

    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    pub fn from_config(configs: Vec<ProviderConfig>) -> LLMResult<Self> {
        let mut registry = Self::new();

        for config in configs {
            let api_key = if let Some(key) = config.api_key {
                key
            } else if let Some(env_var) = config.api_key_env {
                std::env::var(&env_var)
                    .map_err(|_| LLMError::Api(format!("{} not set", env_var)))?
            } else {
                return Err(LLMError::Api("No api_key or api_key_env specified".to_string()));
            };

            let provider: Arc<dyn LLMProvider> = match config.provider.as_str() {
                "openai" => {
                    let mut p = super::openai::OpenAIProvider::new(api_key);
                    if let Some(url) = config.base_url {
                        p = p.with_base_url(url);
                    }
                    if let Some(model) = config.model {
                        p = p.with_model(model);
                    }
                    Arc::new(p)
                }
                "anthropic" => {
                    let mut p = super::anthropic::AnthropicProvider::new(api_key);
                    if let Some(url) = config.base_url {
                        p = p.with_base_url(url);
                    }
                    if let Some(model) = config.model {
                        p = p.with_model(model);
                    }
                    Arc::new(p)
                }
                other => return Err(LLMError::Api(format!("Unknown provider: {other}"))),
            };

            registry.register(&config.provider, provider);
        }

        Ok(registry)
    }

    pub fn from_env() -> LLMResult<Self> {
        let mut configs = Vec::new();

        if std::env::var("OPENAI_API_KEY").is_ok() {
            configs.push(ProviderConfig {
                provider: "openai".to_string(),
                api_key: None,
                api_key_env: Some("OPENAI_API_KEY".to_string()),
                base_url: std::env::var("OPENAI_API_BASE").ok(),
                model: std::env::var("OPENAI_MODEL").ok(),
            });
        }

        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            configs.push(ProviderConfig {
                provider: "anthropic".to_string(),
                api_key: None,
                api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
                base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
                model: std::env::var("ANTHROPIC_MODEL").ok(),
            });
        }

        if configs.is_empty() {
            return Err(LLMError::Api("No provider configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY".to_string()));
        }

        Self::from_config(configs)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
