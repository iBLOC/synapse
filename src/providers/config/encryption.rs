use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;

use super::store::ConfigError;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn from_password(password: &str) -> Result<Self, ConfigError> {
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        Self::from_password_with_salt(password, &salt)
    }

    pub fn from_password_with_salt(password: &str, salt: &[u8]) -> Result<Self, ConfigError> {
        let mut key = [0u8; KEY_LEN];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut key)
            .map_err(|e| ConfigError::Encryption(format!("argon2: {e}")))?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| ConfigError::Encryption(format!("aes key: {e}")))?;

        Ok(Self { cipher })
    }

    pub fn from_key(key: &[u8; KEY_LEN]) -> Result<Self, ConfigError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| ConfigError::Encryption(format!("aes key: {e}")))?;
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, ConfigError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| ConfigError::Encryption(format!("encrypt: {e}")))?;

        let mut result = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, ConfigError> {
        if data.len() < NONCE_LEN {
            return Err(ConfigError::Decryption("data too short".to_string()));
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| ConfigError::Decryption(format!("decrypt: {e}")))
    }
}

pub fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    key
}

pub fn key_to_hex(key: &[u8; KEY_LEN]) -> String {
    hex::encode(key)
}

pub fn hex_to_key(hex_str: &str) -> Result<[u8; KEY_LEN], ConfigError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| ConfigError::Encryption(format!("hex decode: {e}")))?;
    if bytes.len() != KEY_LEN {
        return Err(ConfigError::Encryption(format!("invalid key length: {} (expected {})", bytes.len(), KEY_LEN)));
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_key();
        let encryptor = Encryptor::from_key(&key).unwrap();

        let plaintext = b"sk-secret-api-key-12345";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let key = generate_key();
        let encryptor = Encryptor::from_key(&key).unwrap();

        let plaintext = b"same input";
        let enc1 = encryptor.encrypt(plaintext).unwrap();
        let enc2 = encryptor.encrypt(plaintext).unwrap();

        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_key_hex_roundtrip() {
        let key = generate_key();
        let hex = key_to_hex(&key);
        let restored = hex_to_key(&hex).unwrap();
        assert_eq!(key, restored);
    }

    #[test]
    fn test_decrypt_tampered_fails() {
        let key = generate_key();
        let encryptor = Encryptor::from_key(&key).unwrap();

        let encrypted = encryptor.encrypt(b"secret").unwrap();
        let mut tampered = encrypted.clone();
        tampered[NONCE_LEN] ^= 0xff;

        assert!(encryptor.decrypt(&tampered).is_err());
    }
}
