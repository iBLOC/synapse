use super::{HttpClient, HttpError, HttpMethod, HttpRequest, HttpResponse};
use async_trait::async_trait;
use wasi::http::types::{
    Fields, IncomingBody, Method, OutgoingBody, OutgoingRequest, RequestOptions, Scheme,
};

pub struct WasiHttpClient;

impl WasiHttpClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HttpClient for WasiHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let req_clone = req.clone();
        tokio::task::spawn_blocking(move || send_blocking(&req_clone))
            .await
            .map_err(|e| HttpError::Request(format!("blocking task panicked: {}", e)))?
    }
}

fn send_blocking(req: &HttpRequest) -> Result<HttpResponse, HttpError> {
    let (scheme, authority, path_with_query) = parse_url(&req.url)?;
    if scheme != "https" && scheme != "http" {
        return Err(HttpError::Request(format!(
            "unsupported scheme: {}",
            scheme
        )));
    }

    let method = match req.method {
        HttpMethod::Get => Method::Get,
        HttpMethod::Post => Method::Post,
        HttpMethod::Put => Method::Put,
        HttpMethod::Delete => Method::Delete,
    };

    let headers = build_headers(&req.headers)?;

    let wasi_req = OutgoingRequest::new(headers);
    let scheme_value = if scheme == "https" {
        Scheme::Https
    } else {
        Scheme::Http
    };
    wasi_req
        .set_scheme(Some(&scheme_value))
        .map_err(|_| HttpError::Request("set_scheme failed".to_string()))?;
    wasi_req
        .set_authority(Some(&authority))
        .map_err(|_| HttpError::Request("set_authority failed".to_string()))?;
    wasi_req
        .set_path_with_query(Some(&path_with_query))
        .map_err(|_| HttpError::Request("set_path_with_query failed".to_string()))?;
    wasi_req
        .set_method(&method)
        .map_err(|_| HttpError::Request("set_method failed".to_string()))?;

    if let Some(body_bytes) = &req.body {
        let body_resource = wasi_req
            .body()
            .map_err(|_| HttpError::Request("body() failed".to_string()))?;
        write_body(&body_resource, body_bytes)?;
    }

    let options = RequestOptions::new();
    let future = wasi::http::outgoing_handler::handle(wasi_req, Some(options))
        .map_err(|e| HttpError::Request(format!("outgoing_handler::handle: {:?}", e)))?;

    let response = poll_response_blocking(&future)?;
    let status = response.status();
    let resp_headers = collect_headers(&response.headers());

    let mut body_bytes = Vec::new();
    let body = response
        .consume()
        .map_err(|_| HttpError::Request("consume() failed".to_string()))?;
    let stream = body
        .stream()
        .map_err(|_| HttpError::Request("body.stream() failed".to_string()))?;
    loop {
        match stream.read(64 * 1024) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                body_bytes.extend_from_slice(&chunk);
            }
            Err(e) => {
                return Err(HttpError::Response(format!("read body: {:?}", e)));
            }
        }
    }

    Ok(HttpResponse {
        status,
        headers: resp_headers,
        body: body_bytes,
    })
}

fn parse_url(url: &str) -> Result<(String, String, String), HttpError> {
    let (scheme, rest) = if let Some(idx) = url.find("://") {
        (&url[..idx], &url[idx + 3..])
    } else {
        return Err(HttpError::Request(format!("Invalid URL: {}", url)));
    };
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(HttpError::Request(format!(
            "URL has empty authority: {}",
            url
        )));
    }
    Ok((scheme.to_string(), authority.to_string(), path.to_string()))
}

fn build_headers(pairs: &[(String, String)]) -> Result<Fields, HttpError> {
    let entries: Vec<(String, Vec<u8>)> = pairs
        .iter()
        .map(|(k, v)| (k.clone(), v.as_bytes().to_vec()))
        .collect();
    Fields::from_list(&entries).map_err(|e| HttpError::Request(format!("from_list: {:?}", e)))
}

fn write_body(body: &OutgoingBody, bytes: &[u8]) -> Result<(), HttpError> {
    use std::io::Write;
    let mut stream = body
        .write()
        .map_err(|_| HttpError::Request("body.write failed".to_string()))?;
    stream
        .write_all(bytes)
        .map_err(|e| HttpError::Request(format!("write_all: {}", e)))?;
    drop(stream);
    Ok(())
}

fn collect_headers(fields: &Fields) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let entries = fields.entries();
    for (name, _value) in entries {
        let name_str = name.clone();
        let values = fields.get(&name_str);
        for v in values {
            let v_str = String::from_utf8_lossy(&v).into_owned();
            out.push((name_str.clone(), v_str));
        }
    }
    out
}

fn poll_response_blocking(
    future: &wasi::http::types::FutureIncomingResponse,
) -> Result<wasi::http::types::IncomingResponse, HttpError> {
    let pollable = future.subscribe();
    loop {
        match future.get() {
            Some(Ok(Ok(response))) => return Ok(response),
            Some(Ok(Err(e))) => {
                return Err(HttpError::Request(format!("response error: {:?}", e)));
            }
            Some(Err(())) => {
                return Err(HttpError::Request("future error".to_string()));
            }
            None => {
                let _ready = wasi::io::poll::poll(&[&pollable]);
            }
        }
    }
}
