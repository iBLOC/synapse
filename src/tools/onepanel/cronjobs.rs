use super::client::{OnePanelClient, OnePanelError};

pub struct CronjobsModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> CronjobsModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn search(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/cronjobs/search", serde_json::json!({
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_info(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/cronjobs/load/info", serde_json::json!({
            "id": id,
        })).await
    }

    pub async fn next_run(&self, id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/cronjobs/next", serde_json::json!({
            "id": id,
        })).await
    }

    pub async fn search_records(
        &self,
        id: u64,
        page: u32,
        page_size: u32,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/cronjobs/search/records", serde_json::json!({
            "id": id,
            "page": page,
            "pageSize": page_size,
        })).await
    }

    pub async fn get_record_log(&self, record_id: u64) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/cronjobs/records/log", serde_json::json!({
            "id": record_id,
        })).await
    }

    pub async fn get_script_options(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/cronjobs/script/options").await
    }
}
