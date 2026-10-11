# 将 Synapse Agent 编译为 wasm32-wasip2

> Engineering practice guide. Synapse-supervised agents are
> compiled to a single `.wasm` module per agent and loaded by the
> supervisor (Prefrontal or any other layer-3 project).
> 工程实践指南。由 Synapse supervisor 托管的 agent，每个 agent 编译为单个 `.wasm` module，由 supervisor（Prefrontal 或其他 layer-3 项目）加载。

## 0. 为什么选 `wasm32-wasip2`？

| 目标 | HTTP | 线程 | 选择 |
|---|---|---|---|
| `wasm32-unknown-unknown` | 无 | 无（没有 `tokio`） | ❌ |
| `wasm32-wasi` | 无（没有 `wasi:http`） | 有限 | ❌ |
| `wasm32-wasip1` | 仅 `wasi:http` | 有限 | 可用 |
| **`wasm32-wasip2`** | **`wasi:http` + 组件 + 线程** | **是** | ✅ 选定** |

`wasip2` 是唯一能给我们以下能力的 target：

  - `wasi:http` 出站流量（LLM API 调用）
  - 与 `tokio` 预期一致的稳定线程模型
  - `synapse:host/*` 自定义 import 所需的组件化 import

## 1. 一次性 workspace 搭建

每个 Synapse agent 都位于**自己的 Cargo workspace**，这样产出的 `.wasm` 拥有干净的依赖闭包。`synapse/templates/agent/` 的模板（见 `2026-07-01-project-boundary.md`）已经帮你搭好骨架。

最小目录布局：

```
my-agent/
├── Cargo.toml
├── src/
│   └── lib.rs
└── wit/                         # optional WIT definition
    └── package.wit
```

```toml
[package]
name = "my-agent"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]          # MUST be cdylib for wasm32-wasip2

[dependencies]
synapse = { version = "0.1", default-features = false, features = ["wasm"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wit-bindgen = "0.30"             # for synapse:host/* ABI generation

[profile.release]
opt-level = "z"                  # Optimize for size
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

`src/lib.rs` 必须以 `cdylib` 形式导出 [`wasm-agent-abi.md`](./wasm-agent-abi.md) 中定义的符号：

规范的 stub 写法见 `synapse/templates/agent/src/lib.rs` 的**模板**。

## 2. 构建

```bash
# Install target once per machine
rustup target add wasm32-wasip2

# Debug build
cargo build --target wasm32-wasip2 --release

# Output is at:
#   target/wasm32-wasip2/release/my_agent.wasm
#
# Optionally run `wasm-opt` (binaryen) to shrink further:
#   wasm-opt -Oz target/wasm32-wasip2/release/my_agent.wasm \
#                    -o target/my_agent.optimized.wasm
```

CI 场景：

```bash
cargo build --release --target wasm32-wasip2 && \
  wasm-opt -Oz target/wasm32-wasip2/release/my_agent.wasm \
            -o dist/my_agent.wasm
```

## 3. 校验 WASM image

Synapse CLI 助手（计划加入 `synapse` 二进制，尚未实现）将运行：

```bash
synapse agent verify dist/my_agent.wasm
# Expected output:
#   ✓ ABI version = 1
#   ✓ Exports:
#       agent_card          (fn)
#       stream_run          (fn)
#       cancel_run          (fn)
#       query_task          (fn)
#       subscribe_task      (fn)
#       synapse_abi_version (fn)
#   ✓ Imports:
#       wasi:http/outgoing-handler
#       synapse:host/get_config
#       synapse:host/a2a-call
#   ✓ Linear memory range: 1 page..=64 pages
```

CLI 发布之前，可以用 `wasm-objdump` 或 `wasm-tools` 完成同样的校验：

```bash
wasm-tools component wit target/my_agent.wasm
wasm-tools print target/my_agent.wasm | grep -E '^\s*\(\s*export|\(import'
```

## 4. 从 supervisor 加载

supervisor 侧大致如下：

```rust,no_run
use synapse::wasm::{WasmRuntime, WasmPackage};

let pkg = WasmPackage::load_wasm_file("dist/my_agent.wasm")?;
let runtime = WasmRuntime::new(synapse::wasm::RuntimeConfig::wasip2())?;
let mut instance = runtime.instantiate(pkg)?;

// Provide host imports at instantiation time
instance.provide_import("synapse:host/get_config", my_get_config)?;
instance.provide_import("synapse:host/a2a-call", my_a2a_call)?;

// Drive it via A2A server executor (see synapse::a2a::server)
let executor = SynapseAgentExecutor::new("agent-1".into(), instance);
let handler = DefaultRequestHandler::new(executor, InMemoryTaskStore::new());
let router = a2a_server::router::build(handler, card);
axum::serve(listener, router).await?;
```

## 5. 常见坑

| 坑 | 症状 | 解法 |
|---|---|---|
| 忘了 `crate-type = ["cdylib"]` | `wasm-opt` 抱怨，看不到任何导出 | 加上即可 |
| 用了 `wasm32-wasi` target | `wasi:http` 符号缺失，链接能过但 `sse_post` 运行时 panic | 换成 `wasm32-wasip2` |
| 用了不感知 WASM 的 LLM client（如 `reqwest`） | `reqwest` 链接期报错 | 改用 `synapse::transport::WasiHttpClient` |
| 打包了依赖原生 OpenSSL 的 TLS 库 | 构建报错 | 只用 `rustls-tls` |
| 用了 `std::thread::spawn` | 运行时 panic（WASM 尚不支持线程） | 只用 `tokio::spawn`；WASM 目前是单线程的 |
| 忘了声明宿主 import | `wit-bindgen` 会在链接期失败 | 见模板的 `wit/` 目录 |
| 递归启用 `tokio` feature | 引入不支持的 `process` / `signal` | 只用 `tokio = { features = ["rt", "sync", "time", "macros"] }` |

## 6. 测试 WASM image

在 supervisor（Prefrontal）可用之前，有三种方式可以驱动 agent：

   原生单元测试 —— 通过 mock 宿主 import，让大部分 agent 逻辑以原生方式（`cargo test`）测试。
   直接用 Wasmtime —— 使用 `synapse::wasm::WasmRuntime` 测试 harness（`tests/wasm_runtime_test.rs`），加载 `.wasm` 文件并实际调用导出。
   端到端 A2A 测试 —— 等真正的 supervisor 就位后，发送 `message/send` 并检查响应。

## 7. 版本策略

  Synapse 0.1.x 与 0.2.x 之间 WASM ABI **可能破坏**。1.0 之前请把 agent 的 `synapse` 依赖 pin 紧。
  Synapse 1.0 发布后，WASM ABI 即告稳定，agent 可以继续跨 Synapse minor 版本加载。
