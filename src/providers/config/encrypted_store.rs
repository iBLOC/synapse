use std::path::Path;

use super::encryption::Encryptor;
use super::store::{ConfigError, ConfigStore, ProviderConfig};

pub struct EncryptedFileStore {
    dir: std::path::PathBuf,
    encryptor: Encryptor,
}

impl EncryptedFileStore {
    pub fn new(dir: impl AsRef<Path>, password: &str) -> Result<Self, ConfigError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| ConfigError::Io(format!("create dir: {e}")))?;

        let salt_path = dir.join(".salt");
        let salt = if salt_path.exists() {
            let hex = std::fs::read_to_string(&salt_path)
                .map_err(|e| ConfigError::Io(format!("read salt: {e}")))?;
            hex::decode(hex.trim())
                .map_err(|e| ConfigError::Encryption(format!("hex decode salt: {e}")))?
        } else {
            let mut salt = [0u8; 16];
            use rand::RngCore;
            rand::rngs::OsRng.fill_bytes(&mut salt);
            std::fs::write(&salt_path, hex::encode(salt))
                .map_err(|e| ConfigError::Io(format!("write salt: {e}")))?;
            salt.to_vec()
        };

        let encryptor = Encryptor::from_password_with_salt(password, &salt)?;

        Ok(Self { dir, encryptor })
    }

    pub fn with_key(dir: impl AsRef<Path>, key: &[u8; 32]) -> Result<Self, ConfigError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| ConfigError::Io(format!("create dir: {e}")))?;

        let encryptor = Encryptor::from_key(key)?;
        Ok(Self { dir, encryptor })
    }

    fn config_path(&self, provider_id: &str) -> std::path::PathBuf {
        self.dir.join(format!("{provider_id}.enc"))
    }
}

impl ConfigStore for EncryptedFileStore {
    fn get(&self, provider_id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        let path = self.config_path(provider_id);
        if !path.exists() {
            return Ok(None);
        }
        let encrypted = std::fs::read(&path)
            .map_err(|e| ConfigError::Io(format!("read: {e}")))?;
        let decrypted = self.encryptor.decrypt(&encrypted)?;
        let config: ProviderConfig = serde_json::from_slice(&decrypted)
            .map_err(|e| ConfigError::Serialization(format!("parse: {e}")))?;
        Ok(Some(config))
    }

    fn set(&self, provider_id: &str, config: &ProviderConfig) -> Result<(), ConfigError> {
        let path = self.config_path(provider_id);
        let json = serde_json::to_vec(config)
            .map_err(|e| ConfigError::Serialization(format!("serialize: {e}")))?;
        let encrypted = self.encryptor.encrypt(&json)?;
        std::fs::write(&path, encrypted)
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
            if path.extension().map_or(false, |ext| ext == "enc") {
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
    fn test_encrypted_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = EncryptedFileStore::new(dir.path(), "test-password").unwrap();

        let config = ProviderConfig::new("openai")
            .with_api_key("sk-secret-12345")
            .with_model("gpt-4o");

        store.set("openai", &config).unwrap();

        let loaded = store.get("openai").unwrap().unwrap();
        assert_eq!(loaded.provider, "openai");
        assert_eq!(loaded.api_key, Some("sk-secret-12345".to_string()));
        assert_eq!(loaded.model, Some("gpt-4o".to_string()));

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed.contains(&"openai".to_string()));

        store.delete("openai").unwrap();
        assert!(store.get("openai").unwrap().is_none());
    }

    #[test]
    fn test_encrypted_store_wrong_password() {
        let dir = tempfile::tempdir().unwrap();

        {
            let store = EncryptedFileStore::new(dir.path(), "correct-password").unwrap();
            let config = ProviderConfig::new("openai").with_api_key("sk-secret");
            store.set("openai", &config).unwrap();
        }

        let store = EncryptedFileStore::new(dir.path(), "wrong-password").unwrap();
        assert!(store.get("openai").is_err());
    }

    #[test]
    fn test_encrypted_store_survives_reload() {
        let dir = tempfile::tempdir().unwrap();
        let password = "my-password";

        {
            let store = EncryptedFileStore::new(dir.path(), password).unwrap();
            let config = ProviderConfig::new("anthropic")
                .with_api_key("sk-ant-secret")
                .with_model("claude-3-5-sonnet-20241022");
            store.set("anthropic", &config).unwrap();
        }

        let store = EncryptedFileStore::new(dir.path(), password).unwrap();
        let loaded = store.get("anthropic").unwrap().unwrap();
        assert_eq!(loaded.api_key, Some("sk-ant-secret".to_string()));
    }
}
