# Synapse WASM / A2A 双向下沉设计 Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-26
> **关联项目：** 神经元（Synapse / 本项目）/ 前额叶（PFC / 下游使用方）

## 背景与目标

### Agent 本质
Agent 是**基于大模型 API 调用**实现的一个**独立程序**，核心能力包括：
- **思考（Thinking）**：通过 LLM API 进行推理
- **Function Call**：调用外部工具
- **MCP**：Model Context Protocol 工具集成
- **Skill**：技能模块

agent 是一个 standalone 程序（binary），可以：
- 编译为 native binary（Linux/Mac/Windows）
- 编译为 WASM 模块（被 PFC 加载到隔离 sandbox 中执行）

### 项目职责边界

| 项目 | 职责 | 不负责 |
|------|------|--------|
| **神经元 Synapse（本项目）** | 提供**单 agent** 的工具、模板、协议 | 多 agent 运行时、进程隔离 |
| **前额叶 PFC** | 提供**多 agent 编排 + WASM 隔离运行时** | 单一 agent 的实现细节 |

具体而言：
- **Synapse** 提供：
  - schema（消息、工具、数据结构）
  - components（model/tool/embedding/retriever）
  - adk（agent 抽象、Agent trait、状态管理）
  - compose（Runnable、Graph、Chain、ToolsNode）
  - callbacks（observability）
  - a2a（A2A 协议）
  - **HttpClient transport 抽象**（本次新增 — WASM 兼容）
  - **高层 LLM Client API**（本次新增 — 避免 PFC 重复实现）
- **PFC** 提供：
  - 多 agent 编排（host / plan-execute / multi-agent pattern）
  - WASM sandbox 运行时（用 wasmtime / wasmer 加载 .wasm 模块）
  - 进程级隔离（每个 agent 跑在独立 sandbox）
  - agent 间通过 A2A 协议通讯（这部分 A2A 客户端由 Synapse 提供，但 host 端在 PFC）

### 当前痛点
PFC 加载 WASM agent 时，agent 内部需要：
1. 作为 **A2A 客户端**：调用其他 A2A 端点
2. 作为 **A2A 服务端**：暴露自己作为 A2A 端点被 host / 其他 agent 调用
3. 同样支持 LLM HTTP 调用外部服务（OpenAI/Anthropic/...）

但 Synapse 当前实现有阻塞点：
- `src/protocol/a2a_adapter.rs` 直接使用 `reqwest::Client` — WASM 不可用
- `src/components/model/openai.rs` 直接使用 `reqwest::Client` — WASM 不可用
- PFC 加载的 WASM agent 无法调用 LLM，也无法做 A2A 通信

### 目标
**将 WASM agent 通信所需的"原语"下沉到 Synapse**（HttpClient trait + LLM Client 高层 API + A2A 重构），
使得 PFC 加载的 WASM agent 能直接使用 Synapse API 完成 LLM 调用和 A2A 通信，无需在 WASM 端重新实现 transport 层。

## 设计决策摘要

| 维度 | 决策 |
|------|------|
| 部署目标 | WASI（服务端 WASM 运行时：Wasmtime / Wasmer） |
| 通信方向 | 双向（client + server） |
| 传输层 | HTTP / JSON-RPC（标准 A2A） |
| LLM 调用 | 全部走 HTTP（与 A2A 共享 transport） |
| 抽象方式 | 方案 A'：HttpClient trait + 单一 LLM 客户端 |
| Feature 策略 | 条件编译 `#[cfg(target_arch = "wasm32")]` + type alias `DefaultHttpClient` |

## 架构

### 模块依赖图

```
┌────────────────────────────────────────────────────────┐
│                  Synapse 业务层                         │
│  schema | adk | compose | callbacks | a2a | components│
│       │           │             │                       │
│       └───────────┴─────────────┘                       │
│                    │                                    │
│            ┌───────▼────────┐                           │
│            │   llm 模块     │ ← 新增，提供高层 LLM API │
│            │  (OpenAI/...)  │                           │
│            └───────┬────────┘                           │
│                    │                                    │
│            ┌───────▼────────┐                           │
│            │  components   │                            │
│            │  (BaseChat    │                            │
│            │   Model trait)│                            │
│            └───────┬────────┘                           │
│                    │                                    │
│            ┌───────▼────────┐                           │
│            │  HttpClient   │ ← 新增 trait (transport)  │
│            │  (trait)      │                           │
│            └───────┬────────┘                           │
│                    │                                    │
│       ┌────────────┴────────────┐                       │
│       │                         │                       │
│  ┌────▼─────┐              ┌────▼─────┐                │
│  │ native   │              │  wasi    │                │
│  │ reqwest  │              │ wasi::http│ ← 新增       │
│  │ impl     │              │ impl     │                │
│  └──────────┘              └──────────┘                │
│  (默认)                   (cfg target_arch="wasm32")  │
└────────────────────────────────────────────────────────┘
```

## 模块设计

### 1. `src/transport/`（新模块）

#### `src/transport/mod.rs`
```rust
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("request error: {0}")]
    Request(String),
    #[error("response error: {0}")]
    Response(String),
    #[error("status error: {0}")]
    Status(u16, String),
    #[error("decode error: {0}")]
    Decode(String),
}

pub enum HttpMethod { Get, Post, Put, Delete }

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

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError>;
}

// 高层便捷方法
impl dyn HttpClient {
    pub async fn post_json<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<R, HttpError> { ... }
    // get_json, post_raw, ...
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod wasi;

#[cfg(not(target_arch = "wasm32"))]
pub type DefaultHttpClient = native::ReqwestHttpClient;
#[cfg(target_arch = "wasm32")]
pub type DefaultHttpClient = wasi::WasiHttpClient;
```

#### `src/transport/native.rs`（native 平台）
```rust
use super::{HttpClient, HttpError, HttpRequest, HttpResponse};
use async_trait::async_trait;
use reqwest::Client;

pub struct ReqwestHttpClient {
    client: Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        let mut builder = match req.method {
            HttpMethod::Get => self.client.get(&req.url),
            HttpMethod::Post => self.client.post(&req.url),
            HttpMethod::Put => self.client.put(&req.url),
            HttpMethod::Delete => self.client.delete(&req.url),
        };
        for (k, v) in req.headers { builder = builder.header(&k, &v); }
        if let Some(body) = req.body { builder = builder.body(body); }
        let resp = builder.send().await.map_err(|e| HttpError::Request(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers = resp.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.bytes().await.map_err(|e| HttpError::Response(e.to_string()))?.to_vec();
        Ok(HttpResponse { status, headers, body })
    }
}
```

#### `src/transport/wasi.rs`（WASI 平台）
```rust
use super::{HttpClient, HttpError, HttpRequest, HttpResponse, HttpMethod};
use async_trait::async_trait;
use wasi::http::{outgoing_handler, types::{OutgoingRequest, RequestOptions, Fields, Method}};

pub struct WasiHttpClient;

impl WasiHttpClient {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl HttpClient for WasiHttpClient {
    async fn execute(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        // 1. 构造 wasi OutgoingRequest
        let wasi_req = build_wasi_request(&req)?;
        // 2. 调用 outgoing_handler::handle
        let future_response = outgoing_handler::handle(wasi_req, None)
            .map_err(|e| HttpError::Request(format!("{:?}", e)))?;
        // 3. await response（这是 poll-based 的，需要包成 future）
        let resp = poll_future(future_response).await
            .map_err(|e| HttpError::Response(format!("{:?}", e)))?;
        // 4. 解析 status / headers / body
        // ...
        Ok(HttpResponse { status, headers, body })
    }
}
```

注：wasi::http 是 poll-based（基于 `FutureReader`），需要包装成 `async_trait` 友好的 future。可以用 `wasi-poll` 或手写 pollster 适配。

### 2. `src/llm/`（新模块，高层便捷 API）

#### `src/llm/mod.rs`
```rust
//! LLM 客户端高层 API（避免 PFC 调用方关心 transport）
//!
//! PFC 中只需：
//! ```rust
//! let http = synapse::create_default_http_client();
//! let client = synapse::openai::OpenAIClient::new(http, api_key);
//! ```

pub mod openai;
pub mod anthropic; // 未来

use std::sync::Arc;
use crate::transport::{HttpClient, DefaultHttpClient};

/// 创建当前 target 下的默认 HttpClient
pub fn create_default_http_client() -> Arc<dyn HttpClient> {
    Arc::new(DefaultHttpClient::new())
}

pub use openai::OpenAIClient;
```

#### `src/llm/openai.rs`
```rust
use std::sync::Arc;
use async_trait::async_trait;
use crate::transport::HttpClient;
use crate::components::model::{BaseChatModel, GenerateOptions, ModelResult, ModelError};
use crate::schema::Message;

pub struct OpenAIClient {
    http: Arc<dyn HttpClient>,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(http: Arc<dyn HttpClient>, api_key: impl Into<String>) -> Self {
        Self {
            http,
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
    
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl BaseChatModel for OpenAIClient {
    async fn generate(&self, messages: Vec<Message>, options: GenerateOptions) -> ModelResult<Message> {
        // 使用 self.http.post_json 调用 OpenAI API
        // 解析响应
    }
}
```

### 3. `src/protocol/a2a_adapter.rs` 重构

**改动前：**
```rust
pub struct A2AAdapter {
    client: reqwest::Client,
}
```

**改动后：**
```rust
use std::sync::Arc;
use crate::transport::HttpClient;

pub struct A2AAdapter {
    http: Arc<dyn HttpClient>,
}

impl A2AAdapter {
    pub fn new(http: Arc<dyn HttpClient>) -> Self { Self { http } }
    
    /// 自动选择默认 HttpClient（兼容现有调用方）
    pub fn new_default() -> Self {
        Self { http: crate::llm::create_default_http_client() }
    }
}

#[async_trait]
impl AgentAdapter for A2AAdapter {
    async fn send_task(&self, endpoint: &AgentEndpoint, req: &AgentRequest) -> AdapterResult<AgentResponse> {
        let rpc = build_rpc_request(req);
        let resp: serde_json::Value = self.http.post_json(&endpoint.url, &rpc).await
            .map_err(|e| e.to_string())?;
        // 解析响应...
    }
    // get_task_status, cancel_task, discover_agent 类似
}
```

### 4. `src/components/model/openai.rs` 重构

**改动前：** 直接使用 `reqwest::Client`
**改动后：** 依赖 `HttpClient` trait

```rust
pub struct OpenAIModel {
    http: Arc<dyn HttpClient>,
    api_key: String,
}
```

## Cargo.toml 调整

```toml
[features]
default = []
wasm = []  # PFC 中启用以编译 wasm target

[dependencies]
# 通用依赖
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
async-trait = "0.1"
tokio = { version = "1", features = ["full", "sync"] }

# 按 target 区分
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
reqwest = { version = "0.12", features = ["json"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasi = "0.14"
```

## 错误处理

- `HttpError` 统一抽象所有 transport 层的错误
- A2A adapter 把 `HttpError` 映射到 `AdapterResult`（`String`）保持现有 API
- LLM 把 `HttpError` 映射到 `ModelError` 的 `Api` 变体

## 测试策略

### 1. 单元测试（不依赖 target）
- `HttpRequest` / `HttpResponse` 数据结构测试
- `HttpError` 错误消息测试
- 序列化和反序列化测试

### 2. Native 集成测试
- `tests/transport_native_test.rs`
- 用 `mockito` 或本地 HTTP server 模拟 OpenAI/A2A endpoint
- 验证 `ReqwestHttpClient` 行为

### 3. WASI 集成测试
- 需 wasmtime + wasi http 测试工具
- 提供 `Makefile` 自动化：
  ```makefile
  test-wasm:
      cargo build --target wasm32-wasi
      cargo test --target wasm32-wasi --test transport_wasi_test
  ```
- CI 跑 wasm target 构建和测试

### 4. 现有 A2A 测试
- 保持兼容
- 重构后所有现有测试应继续通过

## 迁移路径

### Phase 1: 引入 transport trait
1. 创建 `src/transport/` 模块
2. 实现 `ReqwestHttpClient`
3. **不**重构任何现有代码 — 仅仅是新增能力

### Phase 2: 重构 A2A adapter
1. `A2AAdapter` 改为接受 `Arc<dyn HttpClient>`
2. 提供 `A2AAdapter::new_default()` 便利构造
3. 更新所有调用方（如果有破坏性改动，提供 deprecated 旧 API）

### Phase 3: 重构 LLM provider
1. `OpenAIModel` 改为接受 `Arc<dyn HttpClient>`
2. 同时在 `src/llm/openai.rs` 提供 `OpenAIClient` 高层 API

### Phase 4: WASI 实现 + 测试
1. 实现 `WasiHttpClient`
2. 写 WASI 集成测试
3. CI 配置 wasm32-wasi target

### Phase 5: PFC 验证
1. PFC 在 native 编译运行现有 e2e 测试
2. PFC 编译 wasm32-wasi 并在 wasmtime 中运行
3. 端到端 A2A 通信测试（native <-> wasm）

## 兼容性

- **不破坏现有 API** — 所有现有 `A2AAdapter::new()` 调用方迁移到 `A2AAdapter::new_default()` 或 `new(http)`
- 内部 `reqwest::Client` 字段变成 `Arc<dyn HttpClient>`，对 `&self` 不可变调用方无影响
- `OpenAIModel` 同样保持向后兼容

## 风险与权衡

| 风险 | 缓解 |
|------|------|
| `wasi::http` API 是 poll-based，与 `async_trait` 集成复杂 | 封装在 `WasiHttpClient` 内部，使用 `wasi-poll` 或自实现 executor 适配 |
| `HttpClient` trait 增加一层间接 | 简单实现，性能损耗可忽略（一次方法调用 + serde） |
| WASI 测试需要 wasmtime 环境 | Makefile + CI 脚本提供一键测试 |
| `BaseTool` 不是 dyn-compatible（已有问题） | 与本次 WASM 工作解耦，独立处理 |

## 不在本次范围（明确边界）

- 浏览器（browser）target — 用户确认仅 WASI
- Edge / Cloudflare Workers target
- MCP 协议（**注意**：MCP 集成属于 Synapse，但本次只下沉 transport，不包含 MCP 协议实现）
- HTTP/2 / WebSocket transport
- WASI 0.3 / component model
- **多 agent 运行时 / agent 编排** — 明确属于 PFC
- **WASM sandbox / 进程隔离** — 明确属于 PFC
- **WASM host function 设计**（PFC 如何 host 一个 .wasm 模块）— 明确属于 PFC

## PFC 与 Synapse 的协作关系

```
┌──────────────────────────────────────────────────────────────┐
│ PFC 进程（native binary）                                    │
│ ├─ wasmtime 运行时                                           │
│ ├─ 多 agent 编排器（host / plan-execute）                     │
│ └─ 实例化 WASM 模块（每个 agent 独立 sandbox）                 │
│         │                                                    │
│         │ host function (wasi::http, wasi::random, ...)       │
│         ▼                                                    │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │  WASM 模块（独立进程隔离）                                │ │
│ │  ┌──────────────────────────────────────────────────┐    │ │
│ │  │ Synapse (compiled to wasm32-wasi)                │    │ │
│ │  │  ├─ Agent (业务逻辑)                              │    │ │
│ │  │  ├─ llm::OpenAIClient ──► HttpClient (trait)    │    │ │
│ │  │  └─ A2AAdapter ─────────► HttpClient (trait)    │    │ │
│ │  │              │                                  │    │ │
│ │  │              ▼                                  │    │ │
│ │  │       WasiHttpClient (wasi::http)              │    │ │
│ │  └─────────────────┬────────────────────────────────┘    │ │
│ └────────────────────┼───────────────────────────────────┘ │
└─────────────────────┼─────────────────────────────────────┘
                      │ wasi::http outgoing
                      ▼
              OpenAI API / Other A2A endpoints
```

**关键点：**
- Synapse 编译为 `.wasm` 模块，被 PFC 加载
- WASM 模块内部使用 `HttpClient` trait
- `WasiHttpClient` 实现调用 `wasi::http`，由 wasmtime 提供 host function
- PFC 的 host function 注册告诉 wasmtime "如何使用 wasi::http"
- 这就是为什么 transport 必须用 trait：wasi::http 由 host（PFC）提供

## 验收标准

### Synapse 侧
- [ ] `cargo build --target x86_64-unknown-linux-gnu` 通过
- [ ] `cargo build --target wasm32-wasi` 通过（首次构建可能需要装 target）
- [ ] 所有现有 47 个测试通过（native）
- [ ] 新增 transport 模块测试 ≥ 5 个（native）
- [ ] `WasiHttpClient` 单元测试通过（用 wasmtime + 本地 mock server）

### PFC 侧（不在本 spec 实现，但作为验收参考）
- [ ] PFC native 加载 Synapse 编译的 native binary，e2e 测试通过
- [ ] PFC 加载 Synapse 编译的 .wasm 模块，e2e 测试通过
- [ ] PFC 加载的 WASM agent 调用外部 OpenAI API 成功
- [ ] PFC 加载的 WASM agent 作为 A2A 客户端调用其他 A2A 端点成功
- [ ] PFC 加载的 WASM agent 作为 A2A 服务端被 host 调用成功

### 端到端
- [ ] native A2A agent <-> wasm A2A agent 通过 A2A 协议双向通信成功（一个在 native 进程，一个在 PFC 管理的 WASM sandbox 中）
