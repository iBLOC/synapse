use super::package::{PackageId, PackageVersion, WasmPackage};
use super::runtime::{ExecutionResult, WasmRuntime};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Stopped,
    Starting,
    Running,
    Paused,
    Reloading,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstance {
    pub id: Uuid,
    pub package_id: PackageId,
    pub version: PackageVersion,
    pub status: InstanceStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub invocation_count: u64,
    pub total_fuel_consumed: u64,
    pub total_memory_mb: u32,
    pub labels: HashMap<String, String>,
}

impl AgentInstance {
    pub fn new(package_id: PackageId, version: PackageVersion) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            package_id,
            version,
            status: InstanceStatus::Stopped,
            started_at: now,
            last_active: now,
            invocation_count: 0,
            total_fuel_consumed: 0,
            total_memory_mb: 0,
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn mark_active(&mut self) {
        self.last_active = chrono::Utc::now();
        self.invocation_count += 1;
    }

    pub fn set_status(&mut self, status: InstanceStatus) {
        self.status = status;
        if matches!(status, InstanceStatus::Running) {
            self.last_active = chrono::Utc::now();
        }
    }

    pub fn record_execution(&mut self, result: &ExecutionResult) {
        self.invocation_count += 1;
        self.total_fuel_consumed = self.total_fuel_consumed.saturating_add(result.fuel_consumed);
        self.total_memory_mb = self.total_memory_mb.max(result.memory_used_mb);
        self.last_active = chrono::Utc::now();
    }

    pub fn full_id(&self) -> String {
        format!("{}@{}#{}", self.package_id, self.version, self.id)
    }
}

pub struct InstanceManager {
    instances: RwLock<HashMap<Uuid, AgentInstance>>,
    runtime: Arc<WasmRuntime>,
}

impl InstanceManager {
    pub fn new(runtime: Arc<WasmRuntime>) -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            runtime,
        }
    }

    pub fn runtime(&self) -> &WasmRuntime {
        &self.runtime
    }

    pub fn create(
        &self,
        package_id: PackageId,
        version: PackageVersion,
    ) -> AgentInstance {
        let instance = AgentInstance::new(package_id, version);
        let id = instance.id;
        self.instances.write().insert(id, instance.clone());
        instance
    }

    pub fn create_with_labels(
        &self,
        package_id: PackageId,
        version: PackageVersion,
        labels: HashMap<String, String>,
    ) -> AgentInstance {
        let mut instance = AgentInstance::new(package_id, version);
        instance.labels = labels;
        let id = instance.id;
        self.instances.write().insert(id, instance.clone());
        instance
    }

    pub fn get(&self, id: &Uuid) -> Option<AgentInstance> {
        self.instances.read().get(id).cloned()
    }

    pub fn list(&self) -> Vec<AgentInstance> {
        let instances = self.instances.read();
        let mut list: Vec<AgentInstance> = instances.values().cloned().collect();
        list.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        list
    }

    pub fn list_by_package(&self, package_id: &PackageId) -> Vec<AgentInstance> {
        self.instances
            .read()
            .values()
            .filter(|i| &i.package_id == package_id)
            .cloned()
            .collect()
    }

    pub fn list_active(&self) -> Vec<AgentInstance> {
        self.instances
            .read()
            .values()
            .filter(|i| matches!(i.status, InstanceStatus::Running | InstanceStatus::Reloading))
            .cloned()
            .collect()
    }

    pub fn start(&self, id: &Uuid, package: &WasmPackage) -> Result<(), String> {
        self.load_module(id, package)?;
        self.update_status(id, InstanceStatus::Running);
        Ok(())
    }

    pub fn stop(&self, id: &Uuid) {
        self.update_status(id, InstanceStatus::Stopped);
    }

    pub fn pause(&self, id: &Uuid) {
        self.update_status(id, InstanceStatus::Paused);
    }

    pub fn resume(&self, id: &Uuid) {
        self.update_status(id, InstanceStatus::Running);
    }

    pub fn reload(&self, id: &Uuid, new_package: &WasmPackage) -> Result<(), String> {
        self.update_status(id, InstanceStatus::Reloading);
        self.load_module(id, new_package)?;
        self.update_status(id, InstanceStatus::Running);
        Ok(())
    }

    pub fn invoke(
        &self,
        id: &Uuid,
        package: &WasmPackage,
        input: &str,
    ) -> Result<ExecutionResult, String> {
        let module = self.load_module(id, package)?;
        let entry = package.metadata.entry.clone();

        let result = self.runtime.execute(&module, &entry, input)?;

        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(id) {
            instance.record_execution(&result);
        }

        Ok(result)
    }

    pub fn remove(&self, id: &Uuid) -> Option<AgentInstance> {
        self.instances.write().remove(id)
    }

    pub fn count(&self) -> usize {
        self.instances.read().len()
    }

    pub fn count_active(&self) -> usize {
        self.instances
            .read()
            .values()
            .filter(|i| matches!(i.status, InstanceStatus::Running))
            .count()
    }

    fn update_status(&self, id: &Uuid, status: InstanceStatus) {
        let mut instances = self.instances.write();
        if let Some(instance) = instances.get_mut(id) {
            instance.set_status(status);
        }
    }

    fn load_module(&self, id: &Uuid, package: &WasmPackage) -> Result<wasmtime::Module, String> {
        let mut instances = self.instances.write();
        let instance = instances
            .get_mut(id)
            .ok_or_else(|| format!("Instance {} not found", id))?;

        instance.set_status(InstanceStatus::Starting);
        self.runtime.load_module(package)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::PackageMetadata;

    fn make_package(id: &str, version: &str) -> WasmPackage {
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "process") (param $p i32) (param $l i32) (result i32)
                    local.get $l
                )
            )
        "#;
        let bytes = wat::parse_str(wat).unwrap();
        let meta = PackageMetadata::new(
            PackageId::new(id),
            PackageVersion::new(1, 0, 0),
            "test",
        );
        WasmPackage::new(meta, bytes)
    }

    fn make_manager() -> (Arc<InstanceManager>, WasmPackage) {
        let runtime = Arc::new(WasmRuntime::new());
        let manager = Arc::new(InstanceManager::new(runtime));
        (manager, make_package("test", "1.0.0"))
    }

    #[test]
    fn test_create_instance() {
        let (manager, _) = make_manager();
        let instance = manager.create(PackageId::new("test"), PackageVersion::new(1, 0, 0));
        assert_eq!(manager.count(), 1);
        assert_eq!(instance.package_id.as_str(), "test");
    }

    #[test]
    fn test_create_with_labels() {
        let (manager, _) = make_manager();
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "prod".to_string());

        let instance = manager.create_with_labels(
            PackageId::new("test"),
            PackageVersion::new(1, 0, 0),
            labels.clone(),
        );

        assert_eq!(instance.labels.get("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_get_and_list() {
        let (manager, _) = make_manager();
        let a = manager.create(PackageId::new("a"), PackageVersion::new(1, 0, 0));
        let b = manager.create(PackageId::new("b"), PackageVersion::new(1, 0, 0));

        assert!(manager.get(&a.id).is_some());
        assert!(manager.get(&b.id).is_some());
        assert_eq!(manager.list().len(), 2);
    }

    #[test]
    fn test_list_by_package() {
        let (manager, _) = make_manager();
        manager.create(PackageId::new("a"), PackageVersion::new(1, 0, 0));
        manager.create(PackageId::new("a"), PackageVersion::new(2, 0, 0));
        manager.create(PackageId::new("b"), PackageVersion::new(1, 0, 0));

        assert_eq!(manager.list_by_package(&PackageId::new("a")).len(), 2);
        assert_eq!(manager.list_by_package(&PackageId::new("b")).len(), 1);
    }

    #[test]
    fn test_start_and_stop() {
        let (manager, pkg) = make_manager();
        let instance = manager.create(pkg.id().clone(), pkg.version().clone());

        manager.start(&instance.id, &pkg).unwrap();
        assert_eq!(manager.get(&instance.id).unwrap().status, InstanceStatus::Running);

        manager.stop(&instance.id);
        assert_eq!(manager.get(&instance.id).unwrap().status, InstanceStatus::Stopped);
    }

    #[test]
    fn test_pause_and_resume() {
        let (manager, pkg) = make_manager();
        let instance = manager.create(pkg.id().clone(), pkg.version().clone());

        manager.start(&instance.id, &pkg).unwrap();
        manager.pause(&instance.id);
        assert_eq!(manager.get(&instance.id).unwrap().status, InstanceStatus::Paused);

        manager.resume(&instance.id);
        assert_eq!(manager.get(&instance.id).unwrap().status, InstanceStatus::Running);
    }

    #[test]
    fn test_reload() {
        let (manager, pkg) = make_manager();
        let instance = manager.create(pkg.id().clone(), pkg.version().clone());

        manager.start(&instance.id, &pkg).unwrap();
        manager.reload(&instance.id, &pkg).unwrap();
        assert_eq!(manager.get(&instance.id).unwrap().status, InstanceStatus::Running);
    }

    #[test]
    fn test_invoke() {
        let (manager, pkg) = make_manager();
        let instance = manager.create(pkg.id().clone(), pkg.version().clone());
        manager.start(&instance.id, &pkg).unwrap();

        let result = manager.invoke(&instance.id, &pkg, "hello").unwrap();
        assert_eq!(result.output, "hello");

        let updated = manager.get(&instance.id).unwrap();
        assert_eq!(updated.invocation_count, 1);
    }

    #[test]
    fn test_remove() {
        let (manager, _) = make_manager();
        let instance = manager.create(PackageId::new("test"), PackageVersion::new(1, 0, 0));

        manager.remove(&instance.id);
        assert_eq!(manager.count(), 0);
        assert!(manager.get(&instance.id).is_none());
    }

    #[test]
    fn test_count_active() {
        let (manager, pkg) = make_manager();
        let a = manager.create(pkg.id().clone(), pkg.version().clone());
        let b = manager.create(pkg.id().clone(), pkg.version().clone());
        manager.start(&a.id, &pkg).unwrap();

        assert_eq!(manager.count_active(), 1);

        manager.start(&b.id, &pkg).unwrap();
        assert_eq!(manager.count_active(), 2);
    }

    #[test]
    fn test_full_id() {
        let instance = AgentInstance::new(
            PackageId::new("my-agent"),
            PackageVersion::new(2, 1, 0),
        );
        let id = instance.id;
        assert!(instance.full_id().starts_with(&format!("my-agent@2.1.0#{}", id)));
    }
}