use std::sync::Arc;

use super::schema::ProviderSchema;
use super::store::{ConfigError, ProviderConfig};
use crate::providers::{LLMError, LLMProvider};
use crate::providers::openai::OpenAIProvider;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::mimo::MimoProvider;

pub struct ProviderBuilder {
    schema: ProviderSchema,
    config: ProviderConfig,
}

impl ProviderBuilder {
    pub fn new(schema: ProviderSchema) -> Self {
        let config = ProviderConfig::new(&schema.id);
        Self { schema, config }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = Some(url.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = Some(model.into());
        self
    }

    pub fn with_field(mut self, field_id: &str, value: serde_json::Value) -> Self {
        self.config.extra.insert(field_id.to_string(), value);
        self
    }

    pub fn with_config(mut self, config: ProviderConfig) -> Self {
        self.config = config;
        self
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for field in &self.schema.fields {
            if field.required {
                let has_value = match field.id.as_str() {
                    "api_key" => self.config.api_key.is_some(),
                    "base_url" => self.config.base_url.is_some(),
                    "model" => self.config.model.is_some(),
                    _ => self.config.extra.contains_key(&field.id),
                };
                if !has_value {
                    return Err(ConfigError::Invalid(format!("{} is required", field.label)));
                }
            }
        }
        Ok(())
    }

    pub fn build_config(self) -> Result<ProviderConfig, ConfigError> {
        self.validate()?;
        Ok(self.config)
    }

    pub fn build(self) -> Result<Arc<dyn LLMProvider>, LLMError> {
        self.validate().map_err(|e| LLMError::Api(e.to_string()))?;

        let provider: Arc<dyn LLMProvider> = match self.schema.id.as_str() {
            "openai" => {
                let api_key = self.config.api_key
                    .ok_or_else(|| LLMError::Api("api_key required".to_string()))?;
                let mut p = OpenAIProvider::new(api_key);
                if let Some(url) = self.config.base_url {
                    p = p.with_base_url(url);
                }
                if let Some(model) = self.config.model {
                    p = p.with_model(model);
                }
                Arc::new(p)
            }
            "anthropic" => {
                let api_key = self.config.api_key
                    .ok_or_else(|| LLMError::Api("api_key required".to_string()))?;
                let mut p = AnthropicProvider::new(api_key);
                if let Some(url) = self.config.base_url {
                    p = p.with_base_url(url);
                }
                if let Some(model) = self.config.model {
                    p = p.with_model(model);
                }
                Arc::new(p)
            }
            "mimo" => {
                let api_key = self.config.api_key
                    .ok_or_else(|| LLMError::Api("api_key required".to_string()))?;
                let mut p = MimoProvider::new(api_key);
                if let Some(model) = self.config.model {
                    p = p.with_model(model);
                }
                Arc::new(p)
            }
            other => return Err(LLMError::Api(format!("unknown provider: {other}"))),
        };

        Ok(provider)
    }
}
