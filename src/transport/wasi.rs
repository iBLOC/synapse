use super::{HttpClient, HttpError, HttpRequest, HttpResponse};
use async_trait::async_trait;

pub struct WasiHttpClient;

impl WasiHttpClient {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl HttpClient for WasiHttpClient {
    async fn execute(&self, _req: HttpRequest) -> Result<HttpResponse, HttpError> {
        Err(HttpError::Request("WasiHttpClient not yet implemented".to_string()))
    }
}
