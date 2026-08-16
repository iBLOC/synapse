use super::client::{OnePanelClient, OnePanelError};

pub struct WebsitesModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> WebsitesModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/websites/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn list(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/websites/list").await
    }

    pub async fn get(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/websites/{id}")).await
    }

    pub async fn get_config(&self, id: u64, config_type: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/websites/{id}/config/{config_type}")).await
    }

    pub async fn get_domains(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/websites/domains/{id}")).await
    }

    pub async fn get_https(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/websites/{id}/https")).await
    }

    pub async fn search_ssl(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/websites/ssl/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_ssl(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.get(&format!("/api/v2/websites/ssl/{id}")).await
    }

    pub async fn read_log(&self, path: &str) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/files/read", serde_json::json!({
            "path": path,
            "page": 1,
            "pageSize": 100,
        })).await
    }
}
