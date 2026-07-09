use super::package::{PackageId, PackageVersion, WasmPackage};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Version not found: {0}@{1}")]
    VersionNotFound(String, String),

    #[error("Version already exists: {0}@{1}")]
    VersionExists(String, String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Integrity check failed for {0}@{1}")]
    IntegrityCheckFailed(String, String),

    #[error("Registry not initialized")]
    NotInitialized,
}

pub type RegistryResult<T> = Result<T, RegistryError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub id: PackageId,
    pub latest_version: PackageVersion,
    pub versions: Vec<PackageVersion>,
    pub author: Option<String>,
    pub description: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

pub struct PackageRegistry {
    registry_dir: Option<std::path::PathBuf>,
    packages: RwLock<HashMap<PackageId, HashMap<PackageVersion, WasmPackage>>>,
}

impl PackageRegistry {
    pub fn new() -> Self {
        Self {
            registry_dir: None,
            packages: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_dir(dir: impl AsRef<Path>) -> Self {
        Self {
            registry_dir: Some(dir.as_ref().to_path_buf()),
            packages: RwLock::new(HashMap::new()),
        }
    }

    pub fn init(&self) -> RegistryResult<()> {
        let dir = self.registry_dir.as_ref().expect("registry_dir not set");
        if dir.exists() {
            std::fs::create_dir_all(dir)
                .map_err(|e| RegistryError::Io(format!("Failed to create dir: {}", e)))?;

            for entry in std::fs::read_dir(dir)
                .map_err(|e| RegistryError::Io(e.to_string()))?
            {
                let entry = entry.map_err(|e| RegistryError::Io(e.to_string()))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("wpkg") {
                    let bytes = std::fs::read(&path)
                        .map_err(|e| RegistryError::Io(e.to_string()))?;
                    let pkg: WasmPackage = serde_json::from_slice(&bytes)
                        .map_err(|e| RegistryError::Serialization(e.to_string()))?;
                    self.store_in_memory(pkg);
                }
            }
        }
        Ok(())
    }

    pub fn store(&self, package: WasmPackage) -> RegistryResult<()> {
        let id = package.id().clone();
        let version = package.version().clone();

        {
            let packages = self.packages.read();
            if let Some(versions) = packages.get(&id) {
                if versions.contains_key(&version) {
                    return Err(RegistryError::VersionExists(
                        id.to_string(),
                        version.to_string(),
                    ));
                }
            }
        }

        if let Some(_) = &self.registry_dir {
            let path = self.package_path(&id, &version);
            std::fs::create_dir_all(path.parent().unwrap())
                .map_err(|e| RegistryError::Io(e.to_string()))?;

            let bytes = serde_json::to_vec(&package)
                .map_err(|e| RegistryError::Serialization(e.to_string()))?;
            std::fs::write(&path, bytes)
                .map_err(|e| RegistryError::Io(e.to_string()))?;
        }

        self.store_in_memory(package);
        Ok(())
    }

    fn store_in_memory(&self, package: WasmPackage) {
        let id = package.id().clone();
        let version = package.version().clone();

        let mut packages = self.packages.write();
        packages
            .entry(id)
            .or_insert_with(HashMap::new)
            .insert(version, package);
    }

    pub fn get(&self, id: &PackageId, version: &PackageVersion) -> Option<WasmPackage> {
        let packages = self.packages.read();
        packages.get(id)?.get(version).cloned()
    }

    pub fn get_latest(&self, id: &PackageId) -> Option<WasmPackage> {
        let packages = self.packages.read();
        let versions = packages.get(id)?;
        versions
            .iter()
            .max_by_key(|(v, _)| *v)
            .map(|(_, p)| p.clone())
    }

    pub fn list(&self) -> Vec<PackageInfo> {
        let packages = self.packages.read();
        let mut infos: Vec<PackageInfo> = packages
            .iter()
            .filter_map(|(id, versions)| {
                let latest = versions.iter().max_by_key(|(v, _)| *v)?.0.clone();
                let mut sorted_versions: Vec<PackageVersion> = versions.keys().cloned().collect();
                sorted_versions.sort();
                sorted_versions.reverse();

                let latest_pkg = versions.get(&latest)?;
                Some(PackageInfo {
                    id: id.clone(),
                    latest_version: latest,
                    versions: sorted_versions,
                    author: latest_pkg.metadata.author.clone(),
                    description: latest_pkg.metadata.description.clone(),
                    installed_at: latest_pkg.metadata.created_at,
                })
            })
            .collect();
        infos.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        infos
    }

    pub fn list_versions(&self, id: &PackageId) -> Vec<PackageVersion> {
        let packages = self.packages.read();
        let mut versions: Vec<PackageVersion> = packages
            .get(id)
            .map(|v| v.keys().cloned().collect())
            .unwrap_or_default();
        versions.sort();
        versions.reverse();
        versions
    }

    pub fn remove(&self, id: &PackageId, version: &PackageVersion) -> RegistryResult<WasmPackage> {
        let mut packages = self.packages.write();
        let versions = packages
            .get_mut(id)
            .ok_or_else(|| RegistryError::PackageNotFound(id.to_string()))?;

        let pkg = versions.remove(version).ok_or_else(|| {
            RegistryError::VersionNotFound(id.to_string(), version.to_string())
        })?;

        if let Some(_) = &self.registry_dir {
            let path = self.package_path(id, version);
            let _ = std::fs::remove_file(&path);
        }

        Ok(pkg)
    }

    pub fn remove_all_versions(&self, id: &PackageId) -> RegistryResult<()> {
        let mut packages = self.packages.write();
        let versions = packages
            .remove(id)
            .ok_or_else(|| RegistryError::PackageNotFound(id.to_string()))?;

        if let Some(_) = &self.registry_dir {
            for version in versions.keys() {
                let path = self.package_path(id, version);
                let _ = std::fs::remove_file(&path);
            }
        }

        Ok(())
    }

    pub fn count(&self) -> usize {
        self.packages.read().len()
    }

    pub fn count_versions(&self, id: &PackageId) -> usize {
        self.packages
            .read()
            .get(id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn has_package(&self, id: &PackageId) -> bool {
        self.packages.read().contains_key(id)
    }

    pub fn has_version(&self, id: &PackageId, version: &PackageVersion) -> bool {
        self.packages
            .read()
            .get(id)
            .map(|v| v.contains_key(version))
            .unwrap_or(false)
    }

    fn package_path(&self, id: &PackageId, version: &PackageVersion) -> std::path::PathBuf {
        let dir = self.registry_dir.as_ref().expect("registry_dir not set");
        dir.join(format!("{}-{}.wpkg", id, version))
    }
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::PackageMetadata;

    fn make_package(id: &str, version: &str) -> WasmPackage {
        let meta = PackageMetadata::new(
            PackageId::new(id),
            PackageVersion::parse(version).unwrap(),
            "test",
        );
        WasmPackage::new(meta, vec![1, 2, 3])
    }

    #[test]
    fn test_registry_store_and_get() {
        let registry = PackageRegistry::new();
        let pkg = make_package("test-agent", "1.0.0");
        registry.store(pkg.clone()).unwrap();

        let retrieved = registry
            .get(&PackageId::new("test-agent"), &PackageVersion::parse("1.0.0").unwrap())
            .unwrap();
        assert_eq!(retrieved.id().as_str(), "test-agent");
    }

    #[test]
    fn test_registry_duplicate_version_rejected() {
        let registry = PackageRegistry::new();
        registry.store(make_package("test", "1.0.0")).unwrap();
        let result = registry.store(make_package("test", "1.0.0"));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_multiple_versions() {
        let registry = PackageRegistry::new();
        registry.store(make_package("test", "1.0.0")).unwrap();
        registry.store(make_package("test", "1.1.0")).unwrap();
        registry.store(make_package("test", "2.0.0")).unwrap();

        assert_eq!(registry.count_versions(&PackageId::new("test")), 3);
        let latest = registry.get_latest(&PackageId::new("test")).unwrap();
        assert_eq!(latest.version().to_string(), "2.0.0");
    }

    #[test]
    fn test_registry_list() {
        let registry = PackageRegistry::new();
        registry.store(make_package("agent-a", "1.0.0")).unwrap();
        registry.store(make_package("agent-b", "2.0.0")).unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_registry_remove_version() {
        let registry = PackageRegistry::new();
        registry.store(make_package("test", "1.0.0")).unwrap();
        registry.store(make_package("test", "2.0.0")).unwrap();

        registry
            .remove(
                &PackageId::new("test"),
                &PackageVersion::parse("1.0.0").unwrap(),
            )
            .unwrap();

        assert_eq!(registry.count_versions(&PackageId::new("test")), 1);
    }

    #[test]
    fn test_registry_remove_all() {
        let registry = PackageRegistry::new();
        registry.store(make_package("test", "1.0.0")).unwrap();
        registry.store(make_package("test", "2.0.0")).unwrap();

        registry.remove_all_versions(&PackageId::new("test")).unwrap();
        assert!(!registry.has_package(&PackageId::new("test")));
    }

    #[test]
    fn test_registry_persistence() {
        let dir = tempdir();
        let registry = PackageRegistry::with_dir(&dir);
        registry.init().unwrap();

        registry.store(make_package("test", "1.0.0")).unwrap();
        drop(registry);

        let registry2 = PackageRegistry::with_dir(&dir);
        registry2.init().unwrap();
        assert!(registry2.has_package(&PackageId::new("test")));
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "prefrontal-registry-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}