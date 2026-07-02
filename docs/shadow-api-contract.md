# Shadow API Contract — for shadow-desktop / shadow-mobile

> **Audience**: anyone building a client that talks to cortex.
> **Base URL**: `http://<cortex-host>:<port>` (default `http://localhost:8443`).
> **Versioning**: prefix `/shadow/v1/`. New versions will use `/shadow/v2/`.
> **Auth (v0.3)**: optional API key. When `cortex.shadow.api_key` is set
> in cortex's config, every shadow request must carry a matching
> `X-Shadow-Key` header. When empty, the shadow API is open (dev mode).

The shadow API is the only cortex surface external clients should use.
It is REST + SSE. Behind it, cortex translates to A2A and forwards to
the appropriate lobe (temporal / parietal / occipital / memory / future
prefrontal registration).

## Authentication

When cortex's config has `cortex.shadow.api_key = "<some-key>"`, every
request to `/shadow/v1/*` must include:

```
X-Shadow-Key: <some-key>
```

The comparison is constant-time (`subtle::ConstantTimeEq`). Missing
header or wrong key returns `401 Unauthorized`. When the config value
is empty, no header is required — this is the dev default.

Cortex-side config (YAML):

```yaml
cortex:
  shadow:
    api_key: "s3cret"  # empty = no auth
```

## Endpoints

### `GET /shadow/v1/agents`

List all registered lobes with their full AgentCard (including
modalities + capabilities).

**Response** (200):
```json
{
  "cortex": {
    "id": "test-shadow",
    "instance_name": "test-shadow-instance",
    "default_agent": "temporal"
  },
  "agents": [
    {
      "name": "temporal",
      "kind": "native",
      "url": "http://temporal.local:8443",
      "enabled": true,
      "skills": ["routing:prefrontal"],
      "agent_card": {
        "name": "temporal",
        "description": "...",
        "url": "http://temporal.local:8443",
        "version": "0.1.0",
        "metadata": {
          "input_modalities": ["text", "voice"],
          "output_modalities": ["text", "voice"],
          "capabilities": [
            "routing:prefrontal",
            "routing:hippocampus",
            "routing:parietal",
            "routing:occipital",
            "voice-emotion"
          ]
        }
      },
      "health_status": "healthy"
    }
  ]
}
```

### `GET /shadow/v1/agents/{name}`

Fetch one lobe by name.

**Errors**:
- `404` if no lobe with that name is registered.

### `POST /shadow/v1/conversations`

Open a new conversation. The conversation is pinned to a single
agent (lobe); the client can handoff mid-conversation by passing
`agent` on subsequent `send_message` calls.

**Request body** (all fields optional):
```json
{
  "agent": "temporal",
  "system": "you are a helpful assistant that speaks in haiku"
}
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `agent` | string | `CORTEX_DEFAULT_AGENT` env, then `"temporal"` | Pinned agent for this conversation. |
| `system` | string | none | Logged into history as an `[agent]` turn. Not yet forwarded to the lobe on the first request (planned for 4.1). |

**Response** (200):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "agent": "temporal",
  "created_at": "2026-07-02T10:00:00Z",
  "updated_at": "2026-07-02T10:00:00Z",
  "history": []
}
```

### `GET /shadow/v1/conversations/{id}`

Read the full conversation including history.

**Errors**:
- `404` if no conversation with that id exists (the store is
  in-memory; conversations do not survive a cortex restart).

### `POST /shadow/v1/conversations/{id}/messages`

Send a text message to the conversation. Returns the agent's reply
in a single JSON response.

**Request body**:
```json
{
  "text": "what's the weather in Tokyo?",
  "agent": "parietal"
}
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `text` | string | (required) | The user's text. Must be non-empty after trim. |
| `agent` | string | conversation's pinned agent | One-shot handoff: if set, this call routes to a different lobe. The conversation's pinned agent does NOT change — only the next call is redirected. |

**Response** (200):
```json
{
  "conversation_id": "550e8400-e29b-41d4-a716-446655440000",
  "agent": "temporal",
  "reply": "It's 28°C and sunny in Tokyo.",
  "state": "COMPLETED",
  "timestamp": 1720000000000
}
```

**Errors**:
- `400` if `text` is empty/whitespace
- `404` if the conversation or the requested `agent` is not found
- `502` if the lobe is reachable but rejects the call

After success, the user turn and the agent reply are both appended to
the conversation's `history` (visible via `GET /conversations/{id}`).

### `POST /shadow/v1/conversations/{id}/messages:stream`

Server-Sent Events version of `messages`. Same request body, but the
response is `text/event-stream` with each event being a JSON object
in `data`.

**Event types**:
- `delta` — partial reply (one or more, the agent's reply accumulated)
- `done` — terminal; the agent has finished, includes the final state + timestamp
- `error` — terminal; the lobe rejected the call (502 from cortex)

**Wire example**:
```
event: delta
data: {"agent": "temporal", "delta": "It's 28"}

event: delta
data: {"agent": "temporal", "delta": "°C..."}

event: done
data: {"conversation_id": "...", "agent": "temporal", "state": "COMPLETED", "timestamp": 1720000000000}
```

**Current behavior**: cortex's streaming path is wired to a2a-rs's
`send_streaming_message` RPC, so the SSE contract above is observed
end-to-end. Each `delta` event carries one token-chunk; the `done`
event fires when the agent's artifact is complete. Empty `delta`
events (from non-text events the SDK ignores) are dropped before
they reach the wire.

A2A note: the a2a-rs SDK translator emits a `Final` event with only
the last chunk's text (it does not accumulate `append=true` chunks).
The shadow handler tracks the accumulated deltas itself and uses
that for the conversation history so the agent turn reflects the
full user-visible reply.

## Modalities & Capabilities

Each lobe publishes what it accepts (`input_modalities`), what it
produces (`output_modalities`), and what it can do (`capabilities`)
in its `agent_card.metadata`. Clients should introspect these before
sending non-text input to avoid 502s.

Current lobe matrix (v0.3):

| Lobe | Inputs | Outputs | Notable capabilities |
|---|---|---|---|
| `memory` | text | text | `memory:recall`, `memory:store`, `memory:list` |
| `temporal` | text, voice | text, voice | `routing:prefrontal`, `voice-emotion` |
| `parietal` | text (JSON sensor payloads) | text | `sensor-ingest`, `anomaly:detect` |
| `occipital` | image | text | `face-recognition`, `face-match:hippocampus` |

## Configuration

Cortex-side env vars (set when starting cortex):

| Var | Default | Effect |
|---|---|---|
| `CORTEX_DEFAULT_AGENT` | `temporal` | The agent used when a client creates a conversation without specifying one. |
| `CORTEX_SKIP_NEURE` | unset | Set to `1` to disable cortex's in-process neure (test/dev). |
| `NEURE_PORT` | (random) | Where cortex's neure listens. Lobe projects call cortex at `/neure/v1/chat/completions`. |

## Lobe registration (for lobe projects, not shadow clients)

Native lobes (temporal, parietal, occipital, memory, prefrontal)
register themselves on startup with cortex at:

```
POST /cortex/lobes/{name}
Content-Type: application/json

{
  "name": "temporal",            // optional; must match path
  "kind": "native",
  "url": "http://temporal.local:8443",
  "agent_card": { ... }
}
```

The path identifies the agent (lobe name = URL segment). When the
body has `name`, it must equal the path segment — this protects
against typos. On 200 OK, the lobe is now discoverable via
`GET /shadow/v1/agents` and routable through the A2A gateway.

Persistence is best-effort: if cortex's database is unreachable
the lobe is still added to the in-memory cache (and reachable
through shadow + A2A) but will not survive a cortex restart.
Re-register after the DB is back.

### Per-lobe URL conventions

The `url` field should point at the A2A endpoint the lobe
exposes. The a2a-rs JSON-RPC transport posts to whatever URL
is registered; the lobe is responsible for mounting its
A2A handler at that path:

| Lobe | URL suffix | Notes |
|---|---|---|
| `temporal`, `parietal`, `occipital`, `memory` | (root) | These use `synapse::a2a::server::A2AServerBuilder` which mounts at `/`. |
| `prefrontal` | `/api/a2a` | prefrontal has its own A2A route at `/api/a2a` (see `prefrontal/src/server/routes.rs`). |

## Conventions

- **All timestamps** are ISO-8601 UTC strings in responses, except
  `SendMessageResponse.timestamp` which is a Unix millis integer
  (matches the A2A `TaskStatus.timestamp`).
- **All ids** are UUIDv4 strings.
- **Errors** always return `{"error": {"code": NNNN, "message": "..."}}`
  with a numeric `code` for client-side dispatch. Common codes:
  - `4001` — bad request (missing field, empty text)
  - `4040` — not found (conversation)
  - `4041` — not found (agent / lobe)
  - `5021` — lobe unreachable / no URL
  - `5022` — lobe rejected the call
  - `5023` — lobe returned malformed response
- **Content-Type**: `application/json` for non-streaming, `text/event-stream`
  for `:stream`.

## Example client (curl)

```bash
# Open a conversation
CONV=$(curl -s -X POST http://localhost:8443/shadow/v1/conversations \
  -H 'content-type: application/json' \
  -d '{"agent": "temporal"}' | jq -r .id)

# Send a message
curl -s -X POST http://localhost:8443/shadow/v1/conversations/$CONV/messages \
  -H 'content-type: application/json' \
  -d '{"text": "hello"}' | jq

# Stream the response
curl -N -X POST http://localhost:8443/shadow/v1/conversations/$CONV/messages:stream \
  -H 'content-type: application/json' \
  -d '{"text": "hello"}'
```

## Future versions

- **v0.4 — Persistent history**: conversations survive cortex restart
  (backed by hippocampus lobe)
- **v0.5 — Media**: `/shadow/v1/media/{lobe}` for WebRTC voice/video
- **v0.6 — Multi-modal input**: POST images/audio as base64 in
  `SendMessageRequest` (forwarded via A2A FilePart)
