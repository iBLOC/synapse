use super::client::{OnePanelClient, OnePanelError};

pub struct AppsModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> AppsModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search_installed(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/apps/installed/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn list_installed(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/apps/installed/list").await
    }

    pub async fn get_installed(&self, install_id: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/apps/installed/info/{install_id}")).await
    }

    pub async fn search_catalog(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/apps/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_app(&self, key: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/apps/{key}")).await
    }

    pub async fn get_services(&self, key: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/apps/services/{key}")).await
    }

    pub async fn load_port(&self, install_id: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/apps/installed/loadport", serde_json::json!({
            "installId": install_id,
        })).await
    }

    pub async fn get_conn_info(&self, install_id: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/apps/installed/conninfo", serde_json::json!({
            "installId": install_id,
        })).await
    }
}
