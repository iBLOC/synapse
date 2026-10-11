---
title: Deploy as WASM Module
---

# Deploy as WASM Module · 部署为 WASM 模块

Compile a Synapse agent to WebAssembly and run it in a sandboxed
environment.
*把 Synapse agent 编译为 WebAssembly，运行在沙箱环境中。*

## Goal · 目标

Build a Synapse agent, compile to `wasm32-wasip2`, and load it in
a `wasmtime` host.
*构建一个 Synapse agent，编译到 `wasm32-wasip2`，再加载进 `wasmtime` host 运行。*

## Step 1: Set Up Toolchain · 第 1 步：配置工具链

```bash
rustup target add wasm32-wasip2
curl https://wasmtime.dev/install.sh | bash
```

## Step 2: Configure Cargo.toml · 第 2 步：配置 Cargo.toml

```toml
[features]
default = []
wasm = []
```

## Step 3: Build the Agent · 第 3 步：构建 Agent

```rust
// lib.rs - a WASM-exported agent
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::agent::react::GraphReActAgent;
use synapse::components::model::OpenAIModel;

pub fn build_agent() -> impl Agent {
    let model = OpenAIModel::new("gpt-4", std::env::var("OPENAI_KEY").unwrap());
    GraphReActAgent::builder()
        .name("wasm-agent")
        .with_model(std::sync::Arc::new(model))
        .build()
        .unwrap()
}

#[no_mangle]
pub extern "C" fn run() {
    let agent = build_agent();
    // ... main loop
}
```

## Step 4: Compile · 第 4 步：编译

```bash
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/my_agent.wasm`
*产物：`target/wasm32-wasip2/release/my_agent.wasm`*

## Step 5: Run in Wasmtime (Host) · 第 5 步：在 Wasmtime 中运行（Host 侧）

```rust
// host.rs - loads the WASM agent
use wasmtime::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "my_agent.wasm")?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let run = instance.get_typed_func::<(), (), _>(&mut store, "run")?;
    run.call(&mut store, ())?;

    Ok(())
}
```

## Why WASM Sandbox? · 为什么用 WASM 沙箱？

When the agent runs in WASM:
*当 agent 跑在 WASM 里时：*

- ✅ No direct network access — All HTTP via host's wasi::http
  *✅ 无直接网络访问 —— 所有 HTTP 都经由 host 的 wasi::http*
- ✅ Memory isolation — Agent can't read host memory
  *✅ 内存隔离 —— agent 读不到 host 的内存*
- ✅ Resource limits — Host controls CPU, memory, time
  *✅ 资源限制 —— CPU、内存、时间都由 host 控制*
- ✅ Auditable — Host can log every HTTP call
  *✅ 可审计 —— host 能记录每一次 HTTP 调用*

## See Also · 另见

- [WASM concepts](/concepts/wasm)
  *WASM 概念*
- [Cross-Process A2A](/howto/a2a-cross-process)
  *跨进程 A2A*
