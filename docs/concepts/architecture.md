---
title: Architecture
---

# Architecture

Synapse is organized as layered modules. Each layer builds on the
abstractions below it.

## Layer Overview

```
┌──────────────────────────────────────────────────────┐
│  ADK (Agent Development Kit)                         │
│    ├─ Agent trait                                   │
│    ├─ ChatModelAgent (LLM + tool calls + ReAct)      │
│    ├─ GraphReActAgent (graph-based orchestration)    │
│    └─ MultiAgent                                    │
│       ├─ RouterSupervisor (LLM-driven)              │
│       ├─ OrchestratorAgent (decompose + parallel)    │
│       ├─ HandoffSupervisor (transfer)                │
│       └─ HybridSupervisor (router + handoff)         │
├──────────────────────────────────────────────────────┤
│  Components                                         │
│    ├─ Model: OpenAI, Anthropic, custom               │
│    │    └─ providers::catalog (19 presets)           │
│    ├─ Tool: BaseTool, InvokableTool, StreamableTool  │
│    ├─ Document: parser, transformer                 │
│    │    ├─ CharacterTextSplitter                     │
│    │    ├─ RecursiveCharacterTextSplitter            │
│    │    ├─ MarkdownHeaderSplitter                    │
│    │    └─ TokenTextSplitter                         │
│    ├─ Embedding: OpenAI                              │
│    ├─ Audio: AsrProvider / TtsProvider               │
│    ├─ Indexer: InMemory (and future: pgvector, etc.)│
│    └─ Retriever: cosine similarity                   │
├──────────────────────────────────────────────────────┤
│  lobe (stateless skill lifecycle glue)              │
│    ├─ StatelessSkill / SkillContext                  │
│    ├─ ProviderSet (LLM + TTS + ASR + Emotion)        │
│    └─ LobeApp (register + serve in 10 lines)         │
├──────────────────────────────────────────────────────┤
│  Transport (cross-platform HTTP)                     │
│    ├─ HttpClient trait                              │
│    ├─ ReqwestHttpClient (native)                    │
│    ├─ WasiHttpClient (wasip2, for WASM sandbox)     │
│    └─ transport::ziti (feature "ziti", OpenZiti)     │
├──────────────────────────────────────────────────────┤
│  MCP (Model Context Protocol)                        │
│    ├─ StdioMcpClient (subprocess + JSON-RPC)         │
│    ├─ HttpMcpClient (HTTP transport)                 │
│    └─ McpServer (server role — expose Synapse tools) │
├──────────────────────────────────────────────────────┤
│  A2A (Agent-to-Agent)                                │
│    └─ A2A Adapter (over HttpClient)                  │
├──────────────────────────────────────────────────────┤
│  ACP (Agent Client Protocol, feature "acp")          │
│    ├─ AcpClient (spawn + initialize + dispatch)      │
│    ├─ AcpAgentSpec (opencode / claude_code / codex)  │
│    └─ translate (SessionUpdate → AgentStreamEvent)   │
├──────────────────────────────────────────────────────┤
│  WASM runtime (package, manager, registry)           │
│    └─ call_entry: handle(json) → json over linear    │
│       memory (closes the WASM-A2A sync bridge)       │
├──────────────────────────────────────────────────────┤
│  Schema (shared types)                              │
│    └─ Message, Content, ToolInfo, ToolCall, etc.     │
└──────────────────────────────────────────────────────┘
```

`acp` and `ziti` are **opt-in feature gates** (`--features acp` / `--features ziti`) — they are not compiled into default builds. `acp` is also non-wasm only.

## Key Design Principles

1. **Trait-based composition** — Every major abstraction is a trait, allowing users to plug in custom implementations.
2. **Async by default** — All I/O is `async`, using `async_trait` for object safety.
3. **Cross-platform** — Same code runs natively and in WASM via conditional `cfg(target_arch)` dependencies.
4. **Reuse run loops** — The ReAct loop in `ChatModelAgent` is reused by RouterSupervisor (tool calls as routing), avoiding duplicated orchestration logic.
5. **No magic** — Each layer's behavior is explicit and traceable.

## Data Flow Example: A ReAct Agent

```
User Input (Vec<Message>)
   │
   ▼
[ChatModelAgent::run_loop]
   │
   │  loop (max_iterations):
   │   │
   │   ▼
   │  [LLM.generate(messages, tools)]
   │   │  (HTTP call to OpenAI / Anthropic / Neuron)
   │   ▼
   │  response: Message { tool_calls, content }
   │   │
   │   ├─ if no tool_calls → return final answer
   │   │
   │   └─ for each tool_call:
   │       │
   │       ▼
   │      [Tool.invoke(args)]
   │       │  - BaseTool, McpTool, A2ATool, etc.
   │       │
   │       ▼
   │      result: String → Message::tool(result, tool_call_id)
   │       │
   │       └─ continue loop
   │
   ▼
Final AgentEvent { output.message, output.action, error? }
```

## MultiAgent Coordination

Each MultiAgent pattern has a distinct coordination signal:

| Pattern | Coordinator | Signal | Who Decides |
|---------|-------------|--------|-------------|
| Router/Supervisor | LLM (host) / LLM（host） | `tool_call` | Top-level LLM |
| Host/Orchestrator | LLM (host) / LLM（host） | `tool_call` (parallel) | Top-level LLM |
| Conversational Handoff | None (rule-based) | `event.action.transfer_to` | Sub-agent / sub-agent |
| Hybrid | LLM + handoff / LLM + handoff | both | LLM picks, sub-agents can transfer |

See [MultiAgent Concepts](/concepts/multiagent) for details.
