---
title: Session Event Log
---

# Session Event Log

Append-only event persistence for agent sessions, inspired by DeepSeek Harness's session architecture.

## Overview

The session event log records every interaction as an immutable event. This enables:

- Durable session history that survives restarts
- Replay and audit capabilities
- Fork/resume from any point

## Types

### SessionEvent

```rust
use synapse::session::{SessionEvent, SessionEventType};

let event = SessionEvent::user_message(session_id, "Hello", 1);
let event = SessionEvent::assistant_message(session_id, "Hi there", 2);
let event = SessionEvent::tool_call(session_id, "search", &args, 3);
```

### SessionEventType

| Type | Description |
|------|-------------|
| `UserMessage` | User input |
| `AssistantMessage` | Model response |
| `AssistantChunk` | Streaming token |
| `ToolCall` | Tool invocation |
| `ToolResult` | Tool output |
| `StepStart/End` | Step boundaries |
| `TurnStart/End` | Turn boundaries |
| `Custom(String)` | Application-defined |

## Storage

### InMemoryEventStore

```rust
use synapse::session::InMemoryEventStore;

let store = InMemoryEventStore::new();
store.create_session(&header).await?;
store.append_event(&event).await?;
```

### FileEventStore

```rust
use synapse::session::FileEventStore;

let store = FileEventStore::new("/var/lib/my-agent/sessions")?;
store.create_session(&header).await?;

// Events persisted as JSON Lines
store.append_event(&event).await?;

// Query with pagination
let events = store.get_events(session_id, from_sequence, Some(10)).await?;
```

## File Format

Each session directory contains:

- `header.json` — Session metadata
- `events.jsonl` — Append-only event log (one JSON per line)

```
/var/lib/my-agent/sessions/
└── 550e8400-e29b-41d4-a716-446655440000/
    ├── header.json
    └── events.jsonl
```
