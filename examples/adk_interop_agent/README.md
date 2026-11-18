# ADK interop agent

An ADK (Google's Agent Development Kit) agent exposed over A2A — the
ADK side of the Synapse <-> ADK interop probe.

## Run it

```bash
pip install "google-adk[a2a]"

# no key, no network: the agent answers from the conversation
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent

# then, from the Synapse side
cargo run --example a2a_interop_client -- http://127.0.0.1:8001/a2a/interop_agent
```

Expected: `PEER_REPLY ADK_OFFLINE_OK INTEROP PROBE` — or
`PEER_REPLY ADK_MODEL_OK INTEROP PROBE` when a model is answering. The two
prefixes differ on purpose: an offline answer must not look like a
model's answer.

## With a real model

```bash
export GOOGLE_API_KEY=...        # or GEMINI_API_KEY
export INTEROP_MODEL=gemini-3.5-flash
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent
```

**The key never goes into a file in this directory.** ADK's own `Gemini`
class builds its `google-genai` client from the environment
(`GOOGLE_API_KEY` first, then `GEMINI_API_KEY`), so the *server process*
carries the key, not the agent.

The agent uses `Gemini` rather than `LiteLlm` on purpose: a Google AI
Studio key authenticates on the native `google-genai` path only. The
OpenAI-compatible surface rejects it with `Invalid Auth key`.

## The card's `url` must match the real mount

ADK copies `agent.json`'s `url` into the card verbatim
(`supportedInterfaces[0].url`) — it does not rewrite it. The JSON-RPC
route is `/a2a/<agent_name>`, so if the card says `http://127.0.0.1:8001/`
every card-following client ends up at the wrong place (`POST /` answers
404, `GET /` answers 405).

`agent.json` therefore points at `http://127.0.0.1:8001/a2a/interop_agent`,
which matches the server above. **Change the port, change both.**

## What the two modes are for

* `offline` (the default) — no model call at all. The protocol shape is
  what is under test: the card, the artifacts, the terminal status. This
  mode runs anywhere, including CI.
* a model name — the agent really calls the model, so the answer really
  comes from the model. That is what proves the model leg end to end.
