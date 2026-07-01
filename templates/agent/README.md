# my-agent

A Synapse agent compiled to wasm32-wasip2. Loadable by any supervisor
implementing the [WASM Agent ABI](../../docs/wasm-agent-abi.md) —
today: Prefrontal.

## Quick start

```bash
# Build the .wasm
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2

# The artifact lands at:
target/wasm32-wasip2/release/my_agent.wasm
```

## Files

| File | Purpose |
|---|---|
| `Cargo.toml` | Manifest, cdylib crate, release profile tuned for size |
| `src/lib.rs` | The agent itself — implement your logic inside `stream_run` |
| `wit/world.wit.json` | Exports/imports contract for the supervisor |

## Replace placeholders

The skeleton emits a single `Final` event. To make a real agent:

1. Edit `src/lib.rs::agent_card` — set `name`, `description`, `skills`.
2. Edit `src/lib.rs::stream_run` — replace the TODO stub with your
   LLM-backed ReAct loop. Use `synapse::stream::StreamReader`,
   `synapse::components::model::OpenAIModel`, etc.
3. Edit `src/lib.rs::load_llm_config` — call `host_get_config()` once
   at startup to receive `llm_base_url`/`llm_api_key`/`llm_model`.
4. Edit `src/lib.rs::cancel_run` and `query_task` to back your in-memory
   task store.

Full guide: `docs/wasm-compilation-guide.md`.
