---
title: Deploy as WASM Module
---

# Deploy as WASM Module

Compile a Synapse agent to WebAssembly and run it in a sandboxed
environment.

## Goal

Build a Synapse agent, compile to `wasm32-wasip2`, and load it in
a `wasmtime` host.

## Step 1: Set Up Toolchain

```bash
rustup target add wasm32-wasip2
curl https://wasmtime.dev/install.sh | bash
```

## Step 2: Configure Cargo.toml

```toml
[features]
default = []
wasm = []
```

## Step 3: Build the Agent

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

## Step 4: Compile

```bash
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/my_agent.wasm`

## Step 5: Run in Wasmtime (Host)

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

## Why WASM Sandbox?

When the agent runs in WASM:

- ✅ No direct network access — All HTTP via host's wasi::http
- ✅ Memory isolation — Agent can't read host memory
- ✅ Resource limits — Host controls CPU, memory, time
- ✅ Auditable — Host can log every HTTP call

## See Also

- [WASM concepts](/concepts/wasm)
- [Cross-Process A2A](/howto/a2a-cross-process)
