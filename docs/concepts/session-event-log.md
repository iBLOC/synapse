---
title: Session Event Log
---

# Session Event Log · 会话事件日志

Append-only event persistence for agent sessions, inspired by DeepSeek Harness's session architecture.
*面向 agent session 的 append-only 事件持久化，灵感来自 DeepSeek Harness 的 session 架构。*

## Overview · 概述

The session event log records every interaction as an immutable event. This enables:
*会话事件日志把每次交互记录为不可变事件，由此带来：*

- Durable session history that survives restarts
  *跨重启依然可用的持久化 session 历史*
- Replay and audit capabilities
  *重放与审计能力*
- Fork/resume from any point
  *从任意点 fork/恢复*

## Types · 类型

### SessionEvent · 会话事件

```rust
use synapse::session::{SessionEvent, SessionEventType};

let event = SessionEvent::user_message(session_id, "Hello", 1);
let event = SessionEvent::assistant_message(session_id, "Hi there", 2);
let event = SessionEvent::tool_call(session_id, "search", &args, 3);
```

### SessionEventType · 事件类型

| Type / 类型 | Description / 说明 |
|------|-------------|
| `UserMessage` | User input / 用户输入 |
| `AssistantMessage` | Model response / 模型响应 |
| `AssistantChunk` | Streaming token / 流式 token |
| `ToolCall` | Tool invocation / 工具调用 |
| `ToolResult` | Tool output / 工具输出 |
| `StepStart/End` | Step boundaries / 步骤边界 |
| `TurnStart/End` | Turn boundaries / 回合边界 |
| `Custom(String)` | Application-defined / 应用层自定义 |

## Storage · 存储

### InMemoryEventStore · 内存事件存储

```rust
use synapse::session::InMemoryEventStore;

let store = InMemoryEventStore::new();
store.create_session(&header).await?;
store.append_event(&event).await?;
```

### FileEventStore · 文件事件存储

```rust
use synapse::session::FileEventStore;

let store = FileEventStore::new("/var/lib/my-agent/sessions")?;
store.create_session(&header).await?;

// Events persisted as JSON Lines
store.append_event(&event).await?;

// Query with pagination
let events = store.get_events(session_id, from_sequence, Some(10)).await?;
```

## File Format · 文件格式

Each session directory contains:
*每个 session 目录包含：*

- `header.json` — Session metadata
  *`header.json` —— session 元数据*
- `events.jsonl` — Append-only event log (one JSON per line)
  *`events.jsonl` —— append-only 事件日志（每行一个 JSON）*

```
/var/lib/my-agent/sessions/
└── 550e8400-e29b-41d4-a716-446655440000/
    ├── header.json
    └── events.jsonl
```
