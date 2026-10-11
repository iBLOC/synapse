---
title: Transport Layer
---

# Transport 层

跨平台 HTTP 抽象。

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

## 实现

| 目标 | 类型 |
|--------|------|
| 原生 | 基于 reqwest |
| WASM | 基于 wasi::http |

## 默认选择

```rust
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = ReqwestHttpClient;

#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = WasiHttpClient;
```

## 相关阅读
