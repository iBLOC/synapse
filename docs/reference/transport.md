---
title: Transport Layer
---

# Transport Layer · Transport 层

Cross-platform HTTP abstraction.
*跨平台 HTTP 抽象。*

## HttpClient Trait · HttpClient trait

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

## Implementations · 实现

| Target / 目标 | Type / 类型 |
|--------|------|
| Native / 原生 | `ReqwestHttpClient` (via reqwest) / 基于 reqwest |
| WASM | `WasiHttpClient` (via wasi::http) / 基于 wasi::http |

## Default Selection · 默认选择

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = WasiHttpClient;
```

## See Also · 相关阅读

- [WASM concepts](/concepts/wasm)
