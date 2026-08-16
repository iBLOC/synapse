use super::client::{OnePanelClient, OnePanelError};

pub struct LogsModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> LogsModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search_operation(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/core/logs/operation", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn search_login(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/core/logs/login", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_system_files(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/logs/system/files").await
    }

    pub async fn read_file(
        &self,
        path: &str,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/files/read", serde_json::json!({
            "path": path,
            "page": page,
            "pageSize": page_size,
        })).await
    }
}
