---
title: Transport Layer
---

# Transport Layer

Cross-platform HTTP abstraction.

## HttpClient Trait

```rust
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError>;
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
```

## Implementations

| Target | Type |
|--------|------|
| Native | `ReqwestHttpClient` (via reqwest) |
| WASM | `WasiHttpClient` (via wasi::http) |

## Default Selection

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = WasiHttpClient;
```

## See Also

- [WASM concepts](/concepts/wasm)
