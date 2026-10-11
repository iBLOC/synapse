# WASM Agent ABI Reference

> The contract between a Synapse-compiled agent (`my_agent.wasm`)
> and the supervisor (Telencephalon, or any layer-3 host) that loads
> and runs it. Synapse guarantees compatibility with this ABI
> across minor (0.x.y) versions; breaking changes require a
> major version bump.

## ABI version

Current: **1**

Each WASM image MUST export a function:

```
synapse_abi_version() -> i32
```

returning the integer `1`. Supervisors reject images whose
returned version differs from the supported range.

## Memory model

The WASM module owns a linear memory. All strings are passed as
`(ptr: i32, len: i32)` pointing into that memory. Strings are
always UTF-8.

The host pre-allocates a **stream response buffer** (default 64 KiB,
configurable per agent) per call to `stream_run`. The agent writes
chunks of `AgentStreamEvent` (CBOR or JSON, see §5) into this
buffer; the host polls and forwards.

## Exports (agent → host)

These are the functions every Synapse agent MUST export.

### `agent_card`

```
agent_card(out_ptr: i32) -> i32
```

Writes an `AgentCard` JSON document into linear memory at `out_ptr`.
Returns the byte length written. Returns `-1` on error.

```json
{
  "name": "weather-agent",
  "description": "Looks up weather forecasts.",
  "version": "1.2.0",
  "skills": [
    {"id": "get_weather", "description": "...", "input_schema": {...}}
  ],
  "default_input_modes": ["text/plain"],
  "default_output_modes": ["text/plain"]
}
```

Conforms to the v1.0 A2A `AgentCard` schema
<https://a2a-protocol.org/latest/specification/#agent-card>.

### `stream_run`

```
stream_run(
  input_ptr: i32,           // JSON encoded AgentInput
  input_len: i32,
  out_handle_ptr: i32       // host writes a u32 stream handle here
) -> i32                     // 0 on success, negative on error
```

Begins a streaming execution. The agent writes a sequence of
`AgentStreamEvent`s into the per-handle response buffer. The host
drains the buffer until the agent writes a terminal `Final`/`Error`
event, then closes the stream handle.

### `cancel_run`

```
cancel_run(handle: u32) -> i32
```

Asks the agent to abort the stream identified by `handle`. The
agent SHOULD emit a final `Error { error: AgentError::Interrupted }`
event and close its end of the buffer.

### `query_task`

```
query_task(
  task_id_ptr: i32,
  task_id_len: i32,
  out_ptr: i32
) -> i32  // returns length of `Task` JSON written to out_ptr
```

Returns the current A2A `Task` state for the given `task_id`, or
`-1` if the task is unknown.

### `subscribe_task`

```
subscribe_task(
  task_id_ptr: i32,
  task_id_len: i32,
  out_handle_ptr: i32      // host writes a u32 stream handle here
) -> i32
```

Like `stream_run` but for an existing task. Returns a stream handle
that yields `StatusUpdateEvent` / `ArtifactUpdateEvent`s.

## Imports (host → agent)

The host MUST provide the following imports.

### `wasi:http/outgoing-handler` (and friends)

Standard WASI HTTP for outbound HTTPS calls. Used by the agent
to reach LLM providers. See <https://github.com/WebAssembly/wasi-http>.

### `synapse:host/get_config`

```
get_config(out_ptr: i32, out_cap: i32) -> i32  // returns length written
```

Returns the agent's runtime configuration as a JSON document:

```json
{
  "llm_base_url": "https://api.openai.com/v1",
  "llm_api_key":  "sk-...",
  "llm_model":    "gpt-4o",
  "llm_extra":    { "organization": "..." },
  "a2a_peers": [
    {"name": "router", "url": "https://router.example.com"}
  ]
}
```

The agent calls this **once at startup** (and re-reads after every
`cancel_run` to pick up rotation). The host controls freshness.

### `synapse:host/a2a_call`

```
a2a_call(
  peer_name_ptr: i32, peer_name_len: i32,
  msg_ptr: i32, msg_len: i32,
  out_ptr: i32, out_cap: i32,
  out_len_ptr: i32
) -> i32  // status code: 0 ok, negative on error
```

Sends an A2A `SendMessageRequest` to the named peer (which must
appear in `a2a_peers` config). The host performs the actual HTTP
call with native `a2a-rs` client SDK, since WASM cannot drive
`reqwest`. Returns the `SendMessageResponse` JSON in `out_ptr`
and the length in `out_len_ptr`.

This is a synchronous call from the agent's perspective. For long
peers the agent SHOULD dispatch it on a `tokio::spawn` and await
its result.

## Event payload format

Events written by the agent into the stream response buffer use
the same `AgentStreamEvent` schema as the in-process Synapse
streaming API:

```json
{
  "event": "TokenDelta",
  "agent": "weather-agent",
  "delta": "The current"
}
```

```json
{
  "event": "ToolStart",
  "agent": "weather-agent",
  "name": "get_weather",
  "args": "{\"city\":\"SF\"}"
}
```

```json
{
  "event": "Final",
  "agent": "weather-agent",
  "message": {"role": "assistant", "content": [{"type": "text", "text": "..."}]}
}
```

See `docs/superpowers/specs/2026-06-30-streaming-design.md §4.2`
for the full variant list. The wire encoding is one JSON object per
event, **newline-delimited** (NDJSON) so a partial write at the end
of the buffer can be retried on the next poll cycle.

## Stability policy

- **Within a single `abi_version`**: the host MUST accept any agent
  that exports the same set of symbols with the same signatures.
- **Across `abi_version`**: the host rejects agents with a different
  version. We provide `synapse migrate <old.wasm> <new.wasm>` (planned,
  not yet implemented) to upgrade agents in place.
- **Deprecated symbols**: when an export is removed, the host
  continues to recognize and call the deprecated symbol during a
  deprecation window of one minor Synapse version, then stops.
