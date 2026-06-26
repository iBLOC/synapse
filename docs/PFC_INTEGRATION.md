# PFC 集成说明

> **目标读者：** PFC（前额叶）项目开发者

本文档说明如何使用 Synapse（神经突触）构建可被 PFC 加载的 WASM agent。

## 三项目关系

| 项目 | 角色 | 对标 |
|------|------|------|
| **神经突触 Synapse**（本项目） | Agent 开发工具包 | LangChain / Eino |
| **神经元 Neuron** | LLM 运行时 | vLLM / Ollama / TGI |
| **前额叶 PFC** | 多 agent 编排 + WASM 隔离运行时 | OpenClaw / EVA |

PFC 加载 Synapse 编译的 agent 模块（native 或 WASM），agent 内部通过 Synapse 的统一 API 调 LLM 和做 A2A 通信。

## 编译 WASM agent

### 1. 安装 wasm32-wasip1 target

```bash
rustup target add wasm32-wasip1
```

### 2. 编译

```bash
cargo build --target wasm32-wasip1 --release
```

产物：`target/wasm32-wasip1/release/your_agent.wasm`

## WASM agent 内部使用 Synapse

```rust
use synapse::llm::{create_default_http_client, OpenAIClient};
use synapse::protocol::A2AAdapter;
use synapse::adk::agent::Agent;
use std::sync::Arc;

fn build_agent() -> impl Agent {
    // 关键：使用 Synapse 的高层 API
    // WASM/native 用同一份代码
    let http = create_default_http_client();
    
    // LLM 调 Neuron（OpenAI 兼容）或外部 API
    let llm = OpenAIClient::new(http.clone(), "neuron-api-key")
        .with_base_url("http://neuron:8080/v1");
    
    // A2A 客户端
    let a2a = A2AAdapter::new(http);
    
    // 业务 agent 构造...
    todo!()
}
```

## PFC host 注册 wasi::http（关键步骤）

WASM agent 内部通过 `HttpClient` trait 发起 HTTP 请求，最终调用 `wasi::http` outgoing handler。这个 handler 必须由 PFC（host）注册到 wasmtime：

```rust
// PFC 侧（不属 Synapse）
use wasmtime::{Engine, Module, Store, WasiCtx, Config};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

let mut config = Config::new();
config.wasm_component_model(true);
let engine = Engine::new(&config)?;

// 创建 WASI 上下文（提供 HTTP 能力）
let wasi = WasiCtxBuilder::new()
    .inherit_stdout()
    .inherit_stderr()
    .build();
let http = WasiHttpCtx::new();
let mut store = Store::new(&engine, (wasi, http));

// 加载 WASM 模块
let module = Module::new(&engine, wasm_bytes)?;

// 实例化（wasmtime 自动注册 wasi:http host function）
let instance = linker.instantiate(&mut store, &module)?;
```

PFC 必须确保 wasi:http host function 已注册到 linker，**否则 Synapse 的 WasiHttpClient 会返回错误**。

## 完整 e2e 测试

完整的 PFC + Synapse + Neuron 三项目联调测试在 PFC 仓库中。

## 关键 API 速查

| 用途 | Synapse API |
|------|-------------|
| 创建 HttpClient | `synapse::llm::create_default_http_client()` |
| OpenAI 兼容 LLM | `synapse::llm::OpenAIClient::new(http, api_key).with_base_url(url)` |
| A2A 客户端 | `synapse::protocol::A2AAdapter::new(http)` |
| Anthropic LLM | `synapse::llm::AnthropicClient::new(http, api_key)` |

## 限制

- 当前 WasiHttpClient 使用 wasi 0.14 (preview 1)，需要 PFC 升级到 wasi 0.2+ (preview 2) 才能真正发出 HTTP 请求
- 浏览器 target 不在支持范围（仅 WASI）
- Legacy providers（OpenAIChatModel, Anthropic azure）是 native only，WASM 用新的 OpenAIModel

## 更多信息

- Spec: `docs/superpowers/specs/2026-06-26-wasm-a2a-design.md`
- Plan: `docs/superpowers/plans/2026-06-26-wasm-a2a-implementation.md`
