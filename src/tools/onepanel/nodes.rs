use super::client::{OnePanelClient, OnePanelError};

pub struct NodesModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> NodesModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/core/nodes/list", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn list_all(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/core/nodes/all").await
    }

    pub async fn list_simple(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/core/nodes/simple/all").await
    }

    pub async fn get_app_update_counts(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/core/xpack/nodes/apps/update").await
    }
}
