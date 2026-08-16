use super::client::{OnePanelClient, OnePanelError};

pub struct MonitoringModule<'a> {
    client: &'a OnePanelClient,
}

impl<'a> MonitoringModule<'a> {
    pub fn new(client: &'a OnePanelClient) -> Self {
        Self { client }
    }

    pub async fn get_os_info(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/dashboard/base/os").await
    }

    pub async fn get_current_node(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/dashboard/current/node").await
    }

    pub async fn get_top_cpu(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/dashboard/current/top/cpu").await
    }

    pub async fn get_top_mem(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/dashboard/current/top/mem").await
    }

    pub async fn search_monitor(
        &self,
        param: serde_json::Value,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/hosts/monitor/search", param).await
    }

    pub async fn get_gpu_options(&self) -> Result<serde_json::Value, OnePanelError> {
        self.client.get("/api/v2/hosts/monitor/gpuoptions").await
    }

    pub async fn search_gpu(
        &self,
        param: serde_json::Value,
    ) -> Result<serde_json::Value, OnePanelError> {
        self.client.post("/api/v2/hosts/monitor/gpu/search", param).await
    }
}
