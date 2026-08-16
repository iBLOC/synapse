use super::client::{OnePanelClient, OnePanelError};

pub struct TaskCenterModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> TaskCenterModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/logs/tasks/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_executing_count(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/logs/tasks/executing/count").await
    }
}
