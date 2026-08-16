use std::path::{Path, PathBuf};
use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::RwLock;

pub use a2a_server::task_store::{TaskStore, TaskVersion};
pub use a2a_server::task_store::InMemoryTaskStore;

use a2a::{A2AError, ListTasksRequest, ListTasksResponse, Task};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredEntry {
    task: Task,
    version: TaskVersion,
}

pub struct FileTaskStore {
    dir: PathBuf,
    index: RwLock<HashMap<String, StoredEntry>>,
}

impl FileTaskStore {
    pub async fn new(dir: impl AsRef<Path>) -> Result<Self, A2AError> {
        let dir = dir.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&dir).await
            .map_err(|e| A2AError::internal(&format!("create dir: {e}")))?;

        let mut index = HashMap::new();
        let mut entries = tokio::fs::read_dir(&dir).await
            .map_err(|e| A2AError::internal(&format!("read dir: {e}")))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| A2AError::internal(&format!("read entry: {e}")))?
        {
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }
            let bytes = tokio::fs::read(&path).await
                .map_err(|e| A2AError::internal(&format!("read file: {e}")))?;
            let stored: StoredEntry = serde_json::from_slice(&bytes)
                .map_err(|e| A2AError::internal(&format!("parse json: {e}")))?;
            index.insert(stored.task.id.clone(), stored);
        }

        Ok(Self {
            dir,
            index: RwLock::new(index),
        })
    }

    fn task_path(&self, task_id: &str) -> PathBuf {
        self.dir.join(format!("{task_id}.json"))
    }

    async fn persist(&self, entry: &StoredEntry) -> Result<(), A2AError> {
        let json = serde_json::to_vec_pretty(entry)
            .map_err(|e| A2AError::internal(&format!("serialize: {e}")))?;
        tokio::fs::write(self.task_path(&entry.task.id), json).await
            .map_err(|e| A2AError::internal(&format!("write file: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl TaskStore for FileTaskStore {
    async fn create(&self, task: Task) -> Result<TaskVersion, A2AError> {
        let mut store = self.index.write().await;
        if store.contains_key(&task.id) {
            return Err(A2AError::internal("task already exists"));
        }
        let entry = StoredEntry { task, version: 1 };
        self.persist(&entry).await?;
        let id = entry.task.id.clone();
        store.insert(id, entry);
        Ok(1)
    }

    async fn update(&self, task: Task) -> Result<TaskVersion, A2AError> {
        let mut store = self.index.write().await;
        let entry = store.get_mut(&task.id)
            .ok_or_else(|| A2AError::task_not_found(&task.id))?;
        entry.version += 1;
        entry.task = task;
        self.persist(entry).await?;
        Ok(entry.version)
    }

    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> {
        let store = self.index.read().await;
        Ok(store.get(task_id).map(|e| e.task.clone()))
    }

    async fn list(&self, req: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> {
        let store = self.index.read().await;
        let mut tasks: Vec<Task> = store.values()
            .filter(|entry| {
                if let Some(ref ctx_id) = req.context_id {
                    if entry.task.context_id != *ctx_id { return false; }
                }
                if let Some(ref status) = req.status {
                    if entry.task.status.state != *status { return false; }
                }
                true
            })
            .map(|e| e.task.clone())
            .collect();

        tasks.sort_by(|a, b| a.id.cmp(&b.id));

        let page_size = match req.page_size {
            Some(size) if size > 0 => size as usize,
            _ => 50,
        };
        let start = req.page_token.as_ref()
            .and_then(|t| t.parse::<usize>().ok())
            .unwrap_or(0);
        let total_size = tasks.len();
        let end = (start + page_size).min(total_size);
        let page = tasks[start..end].to_vec();

        let next_page_token = if end < total_size {
            Some(end.to_string())
        } else {
            None
        };

        let page = page.into_iter().map(|mut task| {
            if let Some(ref hl) = req.history_length {
                let hl = *hl as usize;
                if let Some(ref mut history) = task.history {
                    if hl == 0 {
                        *history = Vec::new();
                    } else if history.len() > hl {
                        let start = history.len() - hl;
                        *history = history[start..].to_vec();
                    }
                }
            }
            task
        }).collect();

        Ok(ListTasksResponse {
            tasks: page,
            next_page_token: next_page_token.unwrap_or_default(),
            page_size: page_size as i32,
            total_size: total_size as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2a::{TaskStatus, TaskState};

    fn make_task(id: &str, ctx: &str, state: TaskState) -> Task {
        Task {
            id: id.to_string(),
            context_id: ctx.to_string(),
            status: TaskStatus {
                state,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_file_store_create_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileTaskStore::new(dir.path()).await.unwrap();

        let task = make_task("t1", "c1", TaskState::Submitted);
        let ver = store.create(task.clone()).await.unwrap();
        assert_eq!(ver, 1);

        let got = store.get("t1").await.unwrap().unwrap();
        assert_eq!(got.id, "t1");
    }

    #[tokio::test]
    async fn test_file_store_persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();

        {
            let store = FileTaskStore::new(dir.path()).await.unwrap();
            store.create(make_task("t1", "c1", TaskState::Submitted)).await.unwrap();
            store.create(make_task("t2", "c2", TaskState::Working)).await.unwrap();
        }

        let store2 = FileTaskStore::new(dir.path()).await.unwrap();
        assert_eq!(store2.get("t1").await.unwrap().unwrap().id, "t1");
        assert_eq!(store2.get("t2").await.unwrap().unwrap().id, "t2");
    }

    #[tokio::test]
    async fn test_file_store_update() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileTaskStore::new(dir.path()).await.unwrap();

        store.create(make_task("t1", "c1", TaskState::Submitted)).await.unwrap();
        let ver = store.update(make_task("t1", "c1", TaskState::Working)).await.unwrap();
        assert_eq!(ver, 2);

        let got = store.get("t1").await.unwrap().unwrap();
        assert_eq!(got.status.state, TaskState::Working);
    }

    #[tokio::test]
    async fn test_file_store_list_filter() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileTaskStore::new(dir.path()).await.unwrap();

        store.create(make_task("t1", "c1", TaskState::Submitted)).await.unwrap();
        store.create(make_task("t2", "c1", TaskState::Working)).await.unwrap();
        store.create(make_task("t3", "c2", TaskState::Submitted)).await.unwrap();

        let req = ListTasksRequest {
            context_id: Some("c1".to_string()),
            status: None,
            page_size: None,
            page_token: None,
            history_length: None,
            status_timestamp_after: None,
            include_artifacts: None,
            tenant: None,
        };
        let resp = store.list(&req).await.unwrap();
        assert_eq!(resp.tasks.len(), 2);
    }
}
