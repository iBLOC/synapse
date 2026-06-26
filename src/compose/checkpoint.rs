//! Graph checkpoint persistence for state recovery.
//!
//! Enables saving and restoring graph execution state for interruption
//! and resumption workflows.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub type CheckpointResult<T> = Result<T, CheckpointError>;

#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("checkpoint not found: {0}")]
    NotFound(String),
    #[error("checkpoint error: {0}")]
    Error(String),
    #[error("serialization error: {0}")]
    SerializationError(String),
}

#[async_trait]
pub trait CheckPointStore: Send + Sync {
    async fn get(&self, id: &str) -> CheckpointResult<Option<Vec<u8>>>;
    async fn set(&self, id: &str, data: Vec<u8>) -> CheckpointResult<()>;
    async fn delete(&self, id: &str) -> CheckpointResult<()>;
    async fn list(&self) -> CheckpointResult<Vec<String>>;
}

pub trait Serializer: Send + Sync {
    fn serialize(&self, value: &dyn std::any::Any) -> CheckpointResult<Vec<u8>>;
    fn deserialize(&self, data: &[u8], value: &mut dyn std::any::Any) -> CheckpointResult<()>;
}

pub struct JsonSerializer;

impl JsonSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for JsonSerializer {
    fn serialize(&self, value: &dyn std::any::Any) -> CheckpointResult<Vec<u8>> {
        let json = value
            .downcast_ref::<serde_json::Value>()
            .ok_or_else(|| CheckpointError::Error("value is not JSON".to_string()))?;
        serde_json::to_vec(json).map_err(|e| CheckpointError::SerializationError(e.to_string()))
    }

    fn deserialize(&self, data: &[u8], value: &mut dyn std::any::Any) -> CheckpointResult<()> {
        let json: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;
        let target = value
            .downcast_mut::<serde_json::Value>()
            .ok_or_else(|| CheckpointError::Error("target is not JSON".to_string()))?;
        *target = json;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckPoint {
    pub channels: HashMap<String, ChannelData>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub skip_pre_handler: HashMap<String, bool>,
    #[serde(default)]
    pub rerun_nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub sub_graphs: HashMap<String, CheckPoint>,
    #[serde(default)]
    pub interrupt_id2_addr: HashMap<String, Vec<Address>>,
    #[serde(default)]
    pub interrupt_id2_state: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelData {
    pub values: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub segments: Vec<AddressSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressSegment {
    pub segment_type: String,
    pub id: String,
}

impl CheckPoint {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            inputs: HashMap::new(),
            state: None,
            skip_pre_handler: HashMap::new(),
            rerun_nodes: Vec::new(),
            sub_graphs: HashMap::new(),
            interrupt_id2_addr: HashMap::new(),
            interrupt_id2_state: HashMap::new(),
        }
    }

    pub fn with_state(mut self, state: serde_json::Value) -> Self {
        self.state = Some(state);
        self
    }

    pub fn add_channel(mut self, name: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        self.channels.insert(name.into(), ChannelData { values });
        self
    }
}

impl Default for CheckPoint {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CheckPointer<
    S: Send + Sync + serde::Serialize + serde::de::DeserializeOwned + Clone + 'static,
> {
    store: Arc<dyn CheckPointStore>,
    serializer: Arc<dyn Serializer>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Send + Sync + serde::Serialize + serde::de::DeserializeOwned + Clone + 'static>
    CheckPointer<S>
{
    pub fn new(store: Arc<dyn CheckPointStore>) -> Self {
        Self {
            store,
            serializer: Arc::new(JsonSerializer::new()),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_serializer(mut self, serializer: Arc<dyn Serializer>) -> Self {
        self.serializer = serializer;
        self
    }

    pub async fn save(&self, id: &str, state: &S) -> CheckpointResult<()> {
        let bytes = serde_json::to_vec(state)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;
        self.store.set(id, bytes).await
    }

    pub async fn load(&self, id: &str) -> CheckpointResult<Option<S>> {
        let bytes = self.store.get(id).await?;
        match bytes {
            Some(data) => {
                let state: S = serde_json::from_slice(&data)
                    .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    pub async fn delete(&self, id: &str) -> CheckpointResult<()> {
        self.store.delete(id).await
    }
}

pub struct MemoryCheckPointStore {
    data: Arc<TokioMutex<HashMap<String, Vec<u8>>>>,
}

impl MemoryCheckPointStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryCheckPointStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CheckPointStore for MemoryCheckPointStore {
    async fn get(&self, id: &str) -> CheckpointResult<Option<Vec<u8>>> {
        let data = self.data.lock().await;
        Ok(data.get(id).cloned())
    }

    async fn set(&self, id: &str, data: Vec<u8>) -> CheckpointResult<()> {
        let mut store = self.data.lock().await;
        store.insert(id.to_string(), data);
        Ok(())
    }

    async fn delete(&self, id: &str) -> CheckpointResult<()> {
        let mut store = self.data.lock().await;
        store.remove(id);
        Ok(())
    }

    async fn list(&self) -> CheckpointResult<Vec<String>> {
        let data = self.data.lock().await;
        Ok(data.keys().cloned().collect())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct FileCheckPointStore {
    dir: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileCheckPointStore {
    pub fn new(dir: impl Into<std::path::PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

#[async_trait]
#[cfg(not(target_arch = "wasm32"))]
impl CheckPointStore for FileCheckPointStore {
    async fn get(&self, id: &str) -> CheckpointResult<Option<Vec<u8>>> {
        let path = self.dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(None);
        }
        tokio::fs::read(&path)
            .await
            .map_err(|e| CheckpointError::Error(e.to_string()))
            .map(Some)
    }

    async fn set(&self, id: &str, data: Vec<u8>) -> CheckpointResult<()> {
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| CheckpointError::Error(e.to_string()))?;
        let path = self.dir.join(format!("{}.json", id));
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| CheckpointError::Error(e.to_string()))
    }

    async fn delete(&self, id: &str) -> CheckpointResult<()> {
        let path = self.dir.join(format!("{}.json", id));
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| CheckpointError::Error(e.to_string()))?;
        }
        Ok(())
    }

    async fn list(&self) -> CheckpointResult<Vec<String>> {
        let mut entries = tokio::fs::read_dir(&self.dir)
            .await
            .map_err(|e| CheckpointError::Error(e.to_string()))?;
        let mut ids = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| CheckpointError::Error(e.to_string()))?
        {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    ids.push(name.trim_end_matches(".json").to_string());
                }
            }
        }
        Ok(ids)
    }
}

pub fn migrate_checkpoint_state<F>(data: Vec<u8>, migrate: F) -> CheckpointResult<Vec<u8>>
where
    F: Fn(serde_json::Value) -> CheckpointResult<serde_json::Value>,
{
    let mut checkpoint: CheckPoint = serde_json::from_slice(&data)
        .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;

    if let Some(ref mut state) = checkpoint.state {
        *state = migrate(state.clone())?;
    }

    serde_json::to_vec(&checkpoint).map_err(|e| CheckpointError::SerializationError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestState {
        value: i32,
    }

    #[tokio::test]
    async fn test_memory_store() {
        let store = Arc::new(MemoryCheckPointStore::new());

        store.set("key1", b"data".to_vec()).await.unwrap();

        let result = store.get("key1").await.unwrap();
        assert_eq!(result, Some(b"data".to_vec()));

        store.delete("key1").await.unwrap();
        assert!(store.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_checkpointer() {
        let store = Arc::new(MemoryCheckPointStore::new());
        let pointer = CheckPointer::<TestState>::new(store.clone());

        let state = TestState { value: 42 };
        pointer.save("test", &state).await.unwrap();

        let loaded = pointer.load("test").await.unwrap();
        assert_eq!(loaded, Some(TestState { value: 42 }));
    }

    #[test]
    fn test_checkpoint_struct() {
        let cp = CheckPoint::new()
            .with_state(serde_json::json!({"counter": 10}))
            .add_channel("output", vec![serde_json::json!("result")]);

        assert!(cp.state.is_some());
        assert_eq!(cp.channels.len(), 1);
    }
}
