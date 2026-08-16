use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecuteType {
    Llm,
    Workflow,
    Skill,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub execute_type: ExecuteType,
    pub parent_id: Option<Uuid>,
    pub root_id: Uuid,
    pub depth: i32,
    pub result: Option<String>,
    pub output: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: impl Into<String>, execute_type: ExecuteType) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();
        Self {
            id,
            title: title.into(),
            status: TaskStatus::Pending,
            execute_type,
            parent_id: None,
            root_id: id,
            depth: 0,
            result: None,
            output: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_parent(mut self, parent_id: Uuid, root_id: Uuid, depth: i32) -> Self {
        self.parent_id = Some(parent_id);
        self.root_id = root_id;
        self.depth = depth;
        self
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    pub fn set_result(&mut self, result: impl Into<String>) {
        self.result = Some(result.into());
        self.updated_at = Utc::now();
    }

    pub fn set_output(&mut self, output: serde_json::Value) {
        self.output = Some(output);
        self.updated_at = Utc::now();
    }
}
