use super::client::{OnePanelClient, OnePanelError};

pub struct ContainersModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> ContainersModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/containers/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn list(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/containers/list", serde_json::json!({})).await
    }

    pub async fn get_status(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/containers/status").await
    }

    pub async fn get_limit(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/containers/limit").await
    }

    pub async fn inspect(&self, id: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/containers/inspect", serde_json::json!({
            "id": id,
        })).await
    }

    pub async fn get_stats(&self, id: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/containers/stats/{id}")).await
    }

    pub async fn list_stats(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/containers/list/stats").await
    }

    pub async fn search_log(
        &self,
        id: &str,
        tail: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/containers/search/log?id={id}&tail={tail}")).await
    }
}
