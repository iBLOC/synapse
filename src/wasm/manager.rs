use super::package::{PackageId, PackageVersion, WasmPackage};
use super::registry::{PackageRegistry, RegistryError, RegistryResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOptions {
    pub verify_integrity: bool,
    pub expected_hash: Option<String>,
    pub allow_downgrade: bool,
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            verify_integrity: true,
            expected_hash: None,
            allow_downgrade: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub id: PackageId,
    pub old_version: Option<PackageVersion>,
    pub new_version: PackageVersion,
    pub was_rollback: bool,
}

pub struct PackageManager {
    registry: Arc<PackageRegistry>,
}

impl PackageManager {
    pub fn new(registry: Arc<PackageRegistry>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &PackageRegistry {
        &self.registry
    }

    pub fn install(&self, package: WasmPackage) -> RegistryResult<()> {
        self.registry.store(package)
    }

    pub fn install_from_file(&self, path: impl AsRef<Path>) -> RegistryResult<WasmPackage> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|e| RegistryError::Io(format!("Failed to read file: {}", e)))?;

        let package: WasmPackage = serde_json::from_slice(&bytes)
            .map_err(|e| RegistryError::Serialization(format!("Failed to parse: {}", e)))?;

        self.install(package.clone())?;
        Ok(package)
    }

    pub fn uninstall(&self, id: &PackageId) -> RegistryResult<()> {
        self.registry.remove_all_versions(id)
    }

    pub fn uninstall_version(
        &self,
        id: &PackageId,
        version: &PackageVersion,
    ) -> RegistryResult<WasmPackage> {
        self.registry.remove(id, version)
    }

    pub fn update(
        &self,
        new_package: WasmPackage,
        options: UpdateOptions,
    ) -> RegistryResult<UpdateResult> {
        if options.verify_integrity {
            if let Some(expected) = &options.expected_hash {
                if !new_package.verify_integrity(expected) {
                    return Err(RegistryError::IntegrityCheckFailed(
                        new_package.id().to_string(),
                        new_package.version().to_string(),
                    ));
                }
            }
        }

        let id = new_package.id().clone();
        let new_version = new_package.version().clone();

        let old_version = self.registry.get_latest(&id).map(|p| p.version().clone());

        if let Some(ref old) = old_version {
            if !options.allow_downgrade && new_version < *old {
                return Err(RegistryError::Serialization(format!(
                    "Refusing downgrade from {} to {} (allow_downgrade=false)",
                    old, new_version
                )));
            }
        }

        self.registry.store(new_package)?;

        Ok(UpdateResult {
            id,
            old_version,
            new_version,
            was_rollback: false,
        })
    }

    pub fn rollback(
        &self,
        id: &PackageId,
        target_version: &PackageVersion,
    ) -> RegistryResult<UpdateResult> {
        let old_version = self.registry.get_latest(id).map(|p| p.version().clone());

        let target = self
            .registry
            .get(id, target_version)
            .ok_or_else(|| {
                RegistryError::VersionNotFound(id.to_string(), target_version.to_string())
            })?;

        if self.registry.has_version(id, target_version) && old_version.as_ref() != Some(target_version) {
            self.registry.remove(id, target_version)?;
        }

        self.registry.store(target)?;

        Ok(UpdateResult {
            id: id.clone(),
            old_version,
            new_version: target_version.clone(),
            was_rollback: true,
        })
    }

    pub fn get(&self, id: &PackageId, version: &PackageVersion) -> Option<WasmPackage> {
        self.registry.get(id, version)
    }

    pub fn get_latest(&self, id: &PackageId) -> Option<WasmPackage> {
        self.registry.get_latest(id)
    }

    pub fn list(&self) -> Vec<super::registry::PackageInfo> {
        self.registry.list()
    }

    pub fn list_versions(&self, id: &PackageId) -> Vec<PackageVersion> {
        self.registry.list_versions(id)
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

    fn make_manager() -> PackageManager {
        let registry = Arc::new(PackageRegistry::new());
        PackageManager::new(registry)
    }

    #[test]
    fn test_install_and_get() {
        let manager = make_manager();
        manager.install(make_package("test", "1.0.0")).unwrap();
        let pkg = manager.get_latest(&PackageId::new("test")).unwrap();
        assert_eq!(pkg.version().to_string(), "1.0.0");
    }

    #[test]
    fn test_update() {
        let manager = make_manager();
        manager.install(make_package("test", "1.0.0")).unwrap();

        let result = manager
            .update(make_package("test", "2.0.0"), UpdateOptions::default())
            .unwrap();

        assert_eq!(result.old_version, Some(PackageVersion::new(1, 0, 0)));
        assert_eq!(result.new_version, PackageVersion::new(2, 0, 0));
        assert!(!result.was_rollback);
    }

    #[test]
    fn test_update_refuses_downgrade() {
        let manager = make_manager();
        manager.install(make_package("test", "2.0.0")).unwrap();

        let result = manager.update(make_package("test", "1.0.0"), UpdateOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_allows_downgrade_when_configured() {
        let manager = make_manager();
        manager.install(make_package("test", "2.0.0")).unwrap();

        let options = UpdateOptions {
            allow_downgrade: true,
            ..Default::default()
        };
        let result = manager.update(make_package("test", "1.0.0"), options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rollback() {
        let manager = make_manager();
        manager.install(make_package("test", "1.0.0")).unwrap();
        manager.install(make_package("test", "2.0.0")).unwrap();
        manager.install(make_package("test", "3.0.0")).unwrap();

        let result = manager
            .rollback(
                &PackageId::new("test"),
                &PackageVersion::parse("1.0.0").unwrap(),
            )
            .unwrap();

        assert!(result.was_rollback);
        assert_eq!(result.new_version, PackageVersion::new(1, 0, 0));
    }

    #[test]
    fn test_integrity_check() {
        let manager = make_manager();
        manager.install(make_package("test", "1.0.0")).unwrap();

        let pkg = make_package("test", "2.0.0");
        let hash = pkg.hash();

        let options = UpdateOptions {
            verify_integrity: true,
            expected_hash: Some(hash.clone()),
            ..Default::default()
        };
        let result = manager.update(pkg, options);
        assert!(result.is_ok());

        let bad_pkg = make_package("test", "3.0.0");
        let bad_options = UpdateOptions {
            verify_integrity: true,
            expected_hash: Some("wrong-hash".to_string()),
            ..Default::default()
        };
        let result = manager.update(bad_pkg, bad_options);
        assert!(result.is_err());
    }

    #[test]
    fn test_uninstall() {
        let manager = make_manager();
        manager.install(make_package("test", "1.0.0")).unwrap();
        manager.install(make_package("test", "2.0.0")).unwrap();

        manager.uninstall(&PackageId::new("test")).unwrap();
        assert!(manager.get_latest(&PackageId::new("test")).is_none());
    }

    #[test]
    fn test_install_from_file() {
        let dir = std::env::temp_dir().join(format!(
            "prefrontal-manager-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wpkg");

        let pkg = make_package("from-file", "1.0.0");
        let bytes = serde_json::to_vec(&pkg).unwrap();
        std::fs::write(&path, bytes).unwrap();

        let manager = make_manager();
        let loaded = manager.install_from_file(&path).unwrap();
        assert_eq!(loaded.id().as_str(), "from-file");

        std::fs::remove_dir_all(&dir).ok();
    }
}