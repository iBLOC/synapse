---
title: A2A Protocol
---

# A2A Protocol

A2A (Agent-to-Agent) is a protocol for agents to communicate with
each other across processes or machines. Synapse implements both
A2A client and server.

## Why A2A?

In multi-agent systems, agents often need to:

- Send tasks to specialized remote agents
- Coordinate workflows across process boundaries
- Discover and invoke agent capabilities

A2A provides a standard protocol (JSON over HTTP) for these scenarios.

## Architecture

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Synapse     │  ─────────────────→  │  A2A Server  │
│  A2A Client  │  (JSON-RPC-like)    │  (Other      │
│              │  ←─────────────────  │   process)   │
└──────────────┘                       └──────────────┘
```

## A2A Client

```rust
use synapse::a2a::A2AClient;

let client = A2AClient::new("http://localhost:8080").await?;
let response = client.send_message(message).await?;
```

## A2A Server

Host a Synapse agent as an A2A server:

```rust
use synapse::a2a::server::A2AServerBuilder;
use synapse::a2a::AgentCard;

let card = AgentCard::new("my-agent", "A helpful assistant", "http://0.0.0.0:8080", "1.0");
A2AServerBuilder::new(agent, card)
    .bind("0.0.0.0:8080".parse()?)
    .serve()
    .await?;
```

### Custom Task Store

By default, the server uses `InMemoryTaskStore`. For persistence:

```rust
use synapse::a2a::store::FileTaskStore;

let store = FileTaskStore::new("/var/lib/my-agent/tasks").await?;
A2AServerBuilder::new(agent, card)
    .serve_with_store(store)
    .await?;
```

`FileTaskStore` persists tasks as JSON files and survives restarts.


## Protocol version

Synapse speaks **A2A v1.0**. The version is advertised in the agent
card's `supportedInterfaces[].protocolVersion` and is negotiated per
request through the `A2A-Version` header.

> ⚠️ **Peers must send `A2A-Version: 1.0`.** The A2A v1.0 specification
> (§3.6.1/§3.6.2) says a request *without* that header is interpreted as
> version `0.3` — and v1.0 servers reject `0.3` outright
> (`VERSION_NOT_SUPPORTED`). The Synapse client sends the header
> automatically; a hand-written peer must send it itself.

## Extensions

An agent may declare protocol extensions in its card, and a client may
opt into them per request.

### Declaring (server)

Put the extension URIs in the card's `capabilities.extensions`. The
server publishes them in its agent card **and** uses the same list to
decide what it can actually honour — the two are the same source, so a
card can never advertise something the agent will not do.

```rust
use synapse::a2a::AgentCard;

let mut card = AgentCard::new("my-agent", "A helpful assistant", "http://127.0.0.1:8080", "1.0.0");
card.capabilities.extensions = vec!["https://example.com/ext/v1".to_string()];
```

### Requesting (client)

```rust
use synapse::a2a::client::A2AClient;

let client = A2AClient::with_extensions(
    "http://localhost:8080",
    vec!["https://example.com/ext/v1".to_string()],
)
.await?;
```

The client sends **both** header spellings, so a peer that knows only
one still sees the request:

* `A2A-Extensions` — the A2A v1.0 specification's name.
* `X-A2A-Extensions` — the name Google's ADK still uses.

### Negotiating

The server intersects the requested URIs with the ones it supports and
reports the outcome in the event metadata:

* `activatedExtensions` — requested **and** supported.
* `unsupportedExtensions` — requested but not supported.

Unsupported extensions are reported rather than dropped: a client that
asks for an extension and gets silence cannot tell "not supported" from
"ignored", and those need different reactions.

The negotiation result rides on the **event** metadata, so it appears on
the streaming path (`SendStreamingMessage`). The unary path returns a
`Task` whose metadata is owned by the task store, not by the event.

## Task lifecycle

A task is finished when the server emits a **terminal** event — a `Task`
or a `StatusUpdate` carrying a terminal state (`completed`, `failed`,
`canceled`, `rejected`). An `ArtifactUpdate` is *content*, not a state:
the final answer arrives as an artifact and the task is still open
afterwards.

Synapse's executor emits that terminal event itself. If the agent's
stream ends without a final message, the task is marked **failed**
rather than completed — reporting "completed" with no answer would be a
silent success.

> ⚠️ **This is not cosmetic.** Google ADK's `RemoteA2aAgent` treats a
> task that ends in `submitted` / `working` as *a stream that was cut
> short* and raises an error instead of returning the answer
> (`_UNFINISHED_TASK_STATES` in
> `google/adk/a2a/agent/_remote_a2a_agent.py`). A server that never
> closes its tasks therefore fails every ADK client.

The server also advertises `capabilities.streaming: true` — it really
does serve `SendStreamingMessage`, and a card that claims otherwise
pushes clients onto the unary path, where A2A v1.0 returns the task
reloaded from the store and the event metadata (the extension
handshake) is dropped.

## Interop with other implementations

Two probes ship with the repo:

* `examples/a2a_sdk_probe.py` — drives a live Synapse server with
  Google's own `a2a-sdk` client (the dependency ADK ships with). It
  measures card resolution, the default client path, the persisted
  task, and a negative control with the `A2A-Version` header removed.
* `examples/adk_remote_probe.py` — drives it with ADK's own
  `RemoteA2aAgent`, i.e. the view an ADK user actually gets. **No
  model key is needed**: `RemoteA2aAgent` forwards to the peer, it
  never calls a model.

```bash
cargo run --example a2a_interop_server -- 9099
pip install "google-adk[a2a]"
python examples/a2a_sdk_probe.py
python examples/adk_remote_probe.py
```

`adk_remote_probe.py` also runs a **causal control**: an in-process
fake peer that reproduces the *pre-fix* behaviour (the answer arrives
as an artifact, but the task never reaches a terminal state). ADK then
reports

    A2A response stream ended before task t-pre-fix reached a terminal state.

— the answer *did* arrive, and ADK still failed the whole exchange.
With the terminal event in place the same probe reads
`SYNAPSE_REPLIED_OK` and no error. Same client, same peer code path;
the terminal event is the only variable.

### ADK as the server

`examples/adk_interop_agent/` is a ready-to-run ADK agent, so the other
direction — Synapse as the client, ADK as the server — can be exercised
too:

```bash
pip install "google-adk[a2a]"

# key-free mode: no model call, no network
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent

# then, from the Synapse side
cargo run --example a2a_interop_client -- http://127.0.0.1:8001/a2a/interop_agent
```

The agent answers `ADK_OFFLINE_OK <your text, uppercased>` — or
`ADK_MODEL_OK <your text, uppercased>` when a model is answering. The two
prefixes differ on purpose: an offline answer must not look like a
model's answer.

With a real model, set two environment variables and the agent calls it
for real:

```bash
export GOOGLE_API_KEY=...        # or GEMINI_API_KEY
export INTEROP_MODEL=gemini-3.5-flash
```

**The key never goes into the agent file.** ADK's own `Gemini` class
builds its `google-genai` client from the environment (`GOOGLE_API_KEY`
first, then `GEMINI_API_KEY`), so the *server process* carries the key,
not the agent. And the agent uses `Gemini`, not `LiteLlm`: a Google AI
Studio key authenticates on the native `google-genai` path only — the
OpenAI-compatible surface rejects it with `Invalid Auth key`.

> ⚠️ **The card's `url` must be the real mount point.** ADK copies
> `agent.json`'s `url` into the card verbatim
> (`supportedInterfaces[0].url`); it does not rewrite it. The JSON-RPC
> route is `/a2a/<agent_name>`, so a card that says
> `http://127.0.0.1:8001/` sends every card-following client to the
> wrong place — `POST /` answers 404 and `GET /` answers 405. Keep the
> two in step when you change the port.

### Where the answer lives

A task carries the answer in **`task.artifacts`**. That is what ADK's
own converter reads (`convert_a2a_task_to_event` flattens every
artifact's parts), and it is what Synapse's client reads.

`task.history` is *not* the answer. A peer may put the caller's own
turn there — Synapse's own server does — so "the last history entry"
can be the caller's own question, handed back as if the agent had
answered it.

`status.message` is the answer only in the boundary states
(`input-required`, `auth-required`, or a completed task that carries no
artifacts). In a **failed** task it is the *error*: ADK stamps the
event with an error code and message instead of returning the text as
content. Synapse's client reports a failed, canceled or rejected task as
a failure — handing "401 UNAUTHENTICATED" back as a well-formed reply
would be a silent failure.

> ⚠️ ADK puts the answer in **both** the working-status message and the
> artifact. A client that reads both would show the answer twice, so
> Synapse's streaming path reads the artifact only.
### What the streaming path guarantees

The streaming path has three rules of its own. Each one is a place
where "it looked fine" was hiding a defect:

* **The answer is the whole artifact, not its last chunk.** A peer may
  stream the answer in chunks — the first update creates the artifact
  (`append: false`), the later ones append to it (`append: true`), and
  the last one closes it (`lastChunk: true`). The final message
  carries the accumulated text.
* **A stream that ends is not a task that ended.** The task reaches an
  end only through a terminal state. A stream that simply stops (peer
  crashed, network dropped) leaves the task open — the client reports
  that as a failure, the way ADK's own client does.
* **The answer may arrive in the status message.** Some peers deliver
  it only that way. When no artifact carried any text, the status
  message is the answer.

> ⚠️ ADK puts the answer in **both** the working-status message and the
> artifact. A client that reads both shows the answer twice, so Synapse
> reads the artifact and keeps the status message only as a fallback.

### A peer on this machine

A peer on `127.0.0.1` must not go through the system proxy. reqwest
picks the proxy up from the operating system (on Windows: the
registry's `ProxyServer`), and the system's own bypass list — the
`ProxyOverride` entry that excludes `localhost` and `127.*` — **does
not take effect for loopback**. So on a machine with a system proxy, a
loopback peer is handed to the proxy, which cannot reach the machine's
own loopback interface and answers `502 Bad Gateway` with an empty
body.

Do not lean on that list either: hyper-util reads the registry's
`ProxyOverride` only when **no** `NO_PROXY`/`no_proxy` is set in the
environment — one environment variable silently replaces the whole
registry list.

Synapse therefore decides by URL. `synapse::http_client::client_for`
bypasses the proxy for loopback peers and keeps it for remote ones
(which may well need it) — and every product path that may talk to a
peer on this machine goes through that one module: the A2A client, the
`v1_compat` client, the end-brain registration, the service registry,
the lobe's OpenAI-compatible LLM client, the decision store and the SSE
transport.
### The ADK-specific extension

ADK ships an extension of its own,
`https://google.github.io/adk-docs/a2a/a2a-extension/`. Its meaning is
"run ADK's updated agent executor" — it fixes message duplication,
output misclassification and sub-agent data loss inside ADK's own
executor. Synapse has no ADK executor, so it does **not** claim that
extension. The honest answer is to report it as unsupported, which is
exactly what the negotiation already does:

    unsupportedExtensions: ["https://google.github.io/adk-docs/a2a/a2a-extension/"]

ADK then takes its legacy path — which is the documented path for plain
A2A peers anyway.
## TaskStore Trait

Implement custom persistence by implementing the `TaskStore` trait:

```rust
use synapse::a2a::store::TaskStore;

struct MyCustomStore { ... }

#[async_trait]
impl TaskStore for MyCustomStore {
    async fn create(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn update(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> { ... }
    async fn list(&self, req: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> { ... }
}
```

## See Also

- [A2A Cross-Process how-to](/howto/a2a-cross-process)
- [WASM Sandbox](/concepts/wasm) — A2A in WASM contexts
