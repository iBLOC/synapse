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

| [`a2a_interop_server`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_interop_server.rs) | Hosts a Synapse agent as an A2A v1.0 server so another implementation can call it |
| [`a2a_interop_client`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_interop_client.rs) | Calls a remote A2A server (e.g. Google ADK) and prints its card and reply |

### Interop probes (Python)

These two are **not** cargo targets — they talk to a live server over the
wire and need `pip install "google-adk[a2a]"`.

| Probe | Demonstrates |
|---|---|
| [`a2a_sdk_probe.py`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_sdk_probe.py) | Drives the server with Google's own `a2a-sdk` client — the dependency ADK ships with |
| [`adk_remote_probe.py`](https://github.com/iBLOC/synapse/blob/master/examples/adk_remote_probe.py) | Drives it with ADK's own `RemoteA2aAgent` — the view an ADK user gets; **no model key needed** |

```bash
cargo run --example a2a_interop_server -- 9099
python examples/adk_remote_probe.py
```

### ADK as the server (agent template)

[`adk_interop_agent/`](https://github.com/iBLOC/synapse/blob/master/examples/adk_interop_agent/)
is a runnable ADK agent, so the other direction can be measured too:
Synapse as the client, ADK as the server.

```bash
pip install "google-adk[a2a]"
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent
cargo run --example a2a_interop_client -- http://127.0.0.1:8001/a2a/interop_agent
```

It answers `ADK_OFFLINE_OK <your text>` without calling a model at all — no key,
no network. Set `INTEROP_MODEL` (and `GOOGLE_API_KEY`) to make it call
a real model instead; the key stays in the environment.

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
4. A Python interop probe does not need to compile — add it to the *Interop probes* table instead, and say what it needs (`google-adk[a2a]`).

## Where to Go Next

- [Quick Start](/guide/quick-start) — a guided first agent
- [How-to Guides](/howto/simple-agent) — step-by-step recipes
