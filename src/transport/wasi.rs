use super::{HttpClient, HttpError, HttpMethod, HttpRequest, HttpResponse};
use async_trait::async_trait;

pub struct WasiHttpClient;

impl WasiHttpClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HttpClient for WasiHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let (scheme, authority, path) =
            parse_url(&req.url).map_err(|e| HttpError::Request(format!("URL parse: {}", e)))?;

        if scheme != "https" && scheme != "http" {
            return Err(HttpError::Request(format!(
                "unsupported scheme: {}",
                scheme
            )));
        }

        Err(HttpError::Request(format!(
            "WasiHttpClient on wasip1: HTTP request to {}://{}{} requires wasi:http/outgoing-handler host function. \
             PFC must register this host function via wasmtime linker.define_by_name(&mut store, \"wasi:http/outgoing-handler\", ...) \
             for the wasm module. \
             For a working wasi-http client, use wasi 0.2+ bindings (wasip2) with wasi::http::outgoing_handler::handle().",
            scheme, authority, path
        )))
    }
}

fn parse_url(url: &str) -> Result<(String, String, String), String> {
    let (scheme, rest) = if let Some(idx) = url.find("://") {
        (&url[..idx], &url[idx + 3..])
    } else {
        return Err(format!("Invalid URL: {}", url));
    };

    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    Ok((scheme.to_string(), authority.to_string(), path.to_string()))
}
