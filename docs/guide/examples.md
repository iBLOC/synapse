---
title: Examples
---

# Examples

Runnable examples live in the repo's [`examples/`](https://github.com/iBLOC/synapse/tree/master/examples) directory. All of them compile with `cargo build --examples`; running one needs an OpenAI-compatible endpoint (replace the `sk-...` placeholder with your key).

| Example | Demonstrates |
|---|---|
| [`simple_agent`](https://github.com/iBLOC/synapse/blob/master/examples/simple_agent.rs) | Minimal agent: `OpenAIModel` + `GraphReActAgent`, one prompt in, one answer out |
| [`agent_with_tools`](https://github.com/iBLOC/synapse/blob/master/examples/agent_with_tools.rs) | Custom tool (`BaseTool` + `InvokableTool`) bundled into a `ToolsNode` and attached to the ReAct loop |
| [`react_agent`](https://github.com/iBLOC/synapse/blob/master/examples/react_agent.rs) | Multi-turn conversation — the caller owns the message history across turns |
| [`rag_pipeline`](https://github.com/iBLOC/synapse/blob/master/examples/rag_pipeline.rs) | Chunk → embed → index → retrieve with the in-memory vector store |

## Running

```bash
git clone https://github.com/iBLOC/synapse
cd synapse
cargo run --example simple_agent
```

Point the model at a local OpenAI-compatible server with `.with_base_url("http://localhost:port/v1")` if you don't want to use the hosted API.

## Adding an example

1. Create `examples/<your_example>.rs` — it compiles against the crate like any other target, so `cargo build --examples` gates it in CI.
2. Keep the doc comment at the top explaining what it demonstrates and how to run it.
3. Add a row to the table above in both `docs/guide/examples.md` and `docs/zh/guide/examples.md`.

## Where to Go Next

- [Quick Start](/guide/quick-start) — a guided first agent
- [How-to Guides](/howto/simple-agent) — step-by-step recipes
