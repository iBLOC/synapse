# Compiling a Synapse Agent to wasm32-wasip2

> Engineering practice guide. Synapse-supervised agents are
> compiled to a single `.wasm` module per agent and loaded by the
> supervisor (Prefrontal or any other layer-3 project).

## 0. Why `wasm32-wasip2`?

| Target | HTTP | Threads | Pick? |
|---|---|---|---|
| `wasm32-unknown-unknown` | No | No (no `tokio`) | ❌ |
| `wasm32-wasi` | No (no `wasi:http`) | Limited | ❌ |
| `wasm32-wasip1` | `wasi:http` only | Limited | OK |
| **`wasm32-wasip2`** | **`wasi:http` + components + threading** | **Yes** | **✅ chosen |

`wasip2` is the only target that gives us:

- `wasi:http` outbound (LLM API calls)
- Stable threading model that matches what `tokio` expects
- Componentized imports we need for the `synapse:host/*` custom imports

## 1. One-time workspace setup

Each Synapse agent lives in its **own Cargo workspace** so the
output `.wasm` has a clean dependency closure. The template at
`synapse/templates/agent/` (see `2026-07-01-project-boundary.md`)
skeletons this for you.

Minimal layout:

```
my-agent/
├── Cargo.toml
├── src/
│   └── lib.rs
└── wit/                         # optional WIT definition
    └── package.wit
```

`Cargo.toml`:

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

`src/lib.rs` must `cdylib`-export the symbols defined in
[`wasm-agent-abi.md`](./wasm-agent-abi.md):

- `agent_card`
- `stream_run`
- `cancel_run`
- `query_task`
- `subscribe_task`
- `synapse_abi_version`

See the **template** at `synapse/templates/agent/src/lib.rs` for the
canonical stubs.

## 2. Building

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

For CI:

```bash
cargo build --release --target wasm32-wasip2 && \
  wasm-opt -Oz target/wasm32-wasip2/release/my_agent.wasm \
            -o dist/my_agent.wasm
```

## 3. Verifying the WASM image

Synapse CLI helper (planned for the `synapse` binary, not yet
implemented) will run:

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

Until the CLI ships, the same verification can be done with `wasm-objdump`
or `wasm-tools`:

```bash
wasm-tools component wit target/my_agent.wasm
wasm-tools print target/my_agent.wasm | grep -E '^\s*\(\s*export|\(import'
```

## 4. Loading from a supervisor

The supervisor does roughly:

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

## 5. Common pitfalls

| Pitfall | Symptom | Fix |
|---|---|---|
| Forgot `crate-type = ["cdylib"]` | `wasm-opt` complains, no exports visible | Add it |
| Used `wasm32-wasi` target | `wasi:http` symbols missing, link succeeds but `sse_post` panics at runtime | Switch to `wasm32-wasip2` |
| Used a non-WASM-aware LLM client (e.g. `reqwest`) | Link-time error from `reqwest` | Use `synapse::transport::WasiHttpClient` |
| Bundled a TLS dep that requires native OpenSSL | Build error | Stick to `rustls-tls` only |
| Used `std::thread::spawn` | Runtime panic (threads not yet on WASM) | Use `tokio::spawn` only; WASM is single-threaded for now |
| Forgot to declare host imports | `wit-bindgen` will fail at link time | See template `wit/` directory |
| Recursive enable `tokio` features | Pulls in unsupported `process` / `signal` | Use `tokio = { features = ["rt", "sync", "time", "macros"] }` only |

## 6. Testing the WASM image

Until the supervisor (Prefrontal) is available, agents can be
exercised in three ways:

1. **Native unit tests** — keep most agent logic tested natively
   (`cargo test`) by mocking the host imports.
2. **Wasmtime direct** — use the `synapse::wasm::WasmRuntime`
   test harness (`tests/wasm_runtime_test.rs`) which loads the
   `.wasm` file and exercises the exports.
3. **End-to-end A2A test** — once a real supervisor is around,
   send `message/send` and inspect the response.

## 7. Versioning

- The WASM ABI **may break** between Synapse 0.1.x and 0.2.x.
  Pin your agent's `synapse` dep tightly during pre-1.0.
- Once Synapse 1.0 ships, the WASM ABI is stable and agents will
  continue to load across minor Synapse versions.
