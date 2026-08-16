use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageId(String);

impl PackageId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PackageId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("PackageId cannot be empty".to_string());
        }
        if !s
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(format!(
                "Invalid PackageId '{}': only alphanumeric, '-', '_', '.' allowed",
                s
            ));
        }
        Ok(Self(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
}

impl PackageVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim_start_matches('v');

        let (version_part, pre) = match s.split_once('-') {
            Some((v, p)) => (v, Some(p.to_string())),
            None => (s, None),
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version '{}': expected MAJOR.MINOR.PATCH", s));
        }

        let major = parts[0].parse::<u32>().map_err(|e| format!("Invalid major: {}", e))?;
        let minor = parts[1].parse::<u32>().map_err(|e| format!("Invalid minor: {}", e))?;
        let patch = parts[2].parse::<u32>().map_err(|e| format!("Invalid patch: {}", e))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }

    pub fn next_patch(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            pre: None,
        }
    }
}

impl fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

impl PartialOrd for PackageVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PackageVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl FromStr for PackageVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: PackageId,
    pub version: PackageVersion,
    pub description: String,
    pub author: Option<String>,
    pub entry: String,
    pub fuel_limit: Option<u64>,
    pub memory_limit_mb: Option<u32>,
    pub allowed_hosts: Vec<String>,
    pub required_tools: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl PackageMetadata {
    pub fn new(name: PackageId, version: PackageVersion, description: impl Into<String>) -> Self {
        Self {
            name,
            version,
            description: description.into(),
            author: None,
            entry: "process".to_string(),
            fuel_limit: Some(10_000_000),
            memory_limit_mb: Some(128),
            allowed_hosts: Vec::new(),
            required_tools: Vec::new(),
            labels: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    pub fn with_entry(mut self, entry: impl Into<String>) -> Self {
        self.entry = entry.into();
        self
    }

    pub fn with_fuel_limit(mut self, fuel: u64) -> Self {
        self.fuel_limit = Some(fuel);
        self
    }

    pub fn with_memory_limit(mut self, mb: u32) -> Self {
        self.memory_limit_mb = Some(mb);
        self
    }

    pub fn with_allowed_host(mut self, host: impl Into<String>) -> Self {
        self.allowed_hosts.push(host.into());
        self
    }

    pub fn with_required_tool(mut self, tool: impl Into<String>) -> Self {
        self.required_tools.push(tool.into());
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPackage {
    pub metadata: PackageMetadata,
    pub module: Vec<u8>,
    pub signature: Option<Vec<u8>>,
}

impl WasmPackage {
    pub fn new(metadata: PackageMetadata, module: Vec<u8>) -> Self {
        Self {
            metadata,
            module,
            signature: None,
        }
    }

    pub fn with_signature(mut self, signature: Vec<u8>) -> Self {
        self.signature = Some(signature);
        self
    }

    pub fn id(&self) -> &PackageId {
        &self.metadata.name
    }

    pub fn version(&self) -> &PackageVersion {
        &self.metadata.version
    }

    pub fn size(&self) -> usize {
        self.module.len()
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.module);
        let result = hasher.finalize();
        hex_encode(&result)
    }

    pub fn full_id(&self) -> String {
        format!("{}@{}", self.id(), self.version())
    }

    pub fn verify_integrity(&self, expected_hash: &str) -> bool {
        self.hash() == expected_hash
    }

    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_id_valid() {
        let id = PackageId::from_str("my-agent").unwrap();
        assert_eq!(id.as_str(), "my-agent");
    }

    #[test]
    fn test_package_id_invalid() {
        assert!(PackageId::from_str("").is_err());
        assert!(PackageId::from_str("with space").is_err());
        assert!(PackageId::from_str("with/slash").is_err());
    }

    #[test]
    fn test_package_id_underscore_dash_dot_allowed() {
        assert!(PackageId::from_str("my_agent").is_ok());
        assert!(PackageId::from_str("my-agent").is_ok());
        assert!(PackageId::from_str("my.agent").is_ok());
        assert!(PackageId::from_str("MyAgent123").is_ok());
    }

    #[test]
    fn test_version_parse() {
        let v = PackageVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(!v.is_prerelease());
    }

    #[test]
    fn test_version_with_v_prefix() {
        let v = PackageVersion::parse("v1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_prerelease() {
        let v = PackageVersion::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(v.major, 1);
        assert!(v.is_prerelease());
        assert_eq!(v.pre, Some("alpha.1".to_string()));
    }

    #[test]
    fn test_version_display() {
        let v = PackageVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_display_with_prerelease() {
        let v = PackageVersion {
            major: 1,
            minor: 2,
            patch: 3,
            pre: Some("beta".to_string()),
        };
        assert_eq!(v.to_string(), "1.2.3-beta");
    }

    #[test]
    fn test_version_ordering() {
        let v1 = PackageVersion::new(1, 0, 0);
        let v2 = PackageVersion::new(2, 0, 0);
        let v12 = PackageVersion::new(1, 2, 0);
        let v112 = PackageVersion::new(1, 1, 2);

        assert!(v1 < v2);
        assert!(v1 < v12);
        assert!(v112 < v12);
        assert!(v1 < v112);
    }

    #[test]
    fn test_version_next_patch() {
        let v = PackageVersion::new(1, 2, 3);
        assert_eq!(v.next_patch().to_string(), "1.2.4");
    }

    #[test]
    fn test_package_metadata_builder() {
        let meta = PackageMetadata::new(
            PackageId::new("weather-agent"),
            PackageVersion::new(1, 0, 0),
            "Weather lookup agent",
        )
        .with_author("alice")
        .with_entry("run")
        .with_fuel_limit(5_000_000)
        .with_memory_limit(64)
        .with_allowed_host("api.weather.com")
        .with_required_tool("http")
        .with_label("tier", "production");

        assert_eq!(meta.author, Some("alice".to_string()));
        assert_eq!(meta.entry, "run");
        assert_eq!(meta.fuel_limit, Some(5_000_000));
        assert_eq!(meta.memory_limit_mb, Some(64));
        assert_eq!(meta.allowed_hosts, vec!["api.weather.com"]);
        assert_eq!(meta.required_tools, vec!["http"]);
        assert_eq!(meta.labels.get("tier"), Some(&"production".to_string()));
    }

    #[test]
    fn test_wasm_package_hash() {
        let module = b"\0asm\x01\x00\x00\x00fake wasm bytes";
        let meta = PackageMetadata::new(
            PackageId::new("test"),
            PackageVersion::new(1, 0, 0),
            "Test",
        );
        let pkg = WasmPackage::new(meta, module.to_vec());

        let hash = pkg.hash();
        assert_eq!(hash.len(), 64);
        assert!(pkg.verify_integrity(&hash));
        assert!(!pkg.verify_integrity("invalid_hash"));
    }

    #[test]
    fn test_wasm_package_full_id() {
        let module = b"wasm";
        let meta = PackageMetadata::new(
            PackageId::new("my-agent"),
            PackageVersion::new(2, 1, 0),
            "Test",
        );
        let pkg = WasmPackage::new(meta, module.to_vec());

        assert_eq!(pkg.full_id(), "my-agent@2.1.0");
    }

    #[test]
    fn test_wasm_package_signed() {
        let pkg = WasmPackage::new(
            PackageMetadata::new(PackageId::new("test"), PackageVersion::new(1, 0, 0), "T"),
            vec![],
        );
        assert!(!pkg.is_signed());

        let signed = pkg.with_signature(vec![1, 2, 3]);
        assert!(signed.is_signed());
    }
}