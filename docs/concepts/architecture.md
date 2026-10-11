---
title: Architecture
---

# Architecture · 架构

Synapse is organized as layered modules. Each layer builds on the
abstractions below it.
*Synapse 按分层模块组织，每一层都构建在其下层的抽象之上。*

## Layer Overview · 分层总览

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
*`acp` 和 `ziti` 是**可选 feature 门控**（`--features acp` / `--features ziti`）——默认构建不编译它们。`acp` 还仅在非 wasm 目标可用。*

## Key Design Principles · 核心设计原则

1. **Trait-based composition** — Every major abstraction is a trait, allowing users to plug in custom implementations.
   ***基于 trait 的组合** —— 每个主要抽象都是 trait，用户可以插入自定义实现。*
2. **Async by default** — All I/O is `async`, using `async_trait` for object safety.
   ***默认异步** —— 所有 I/O 都是 `async`，用 `async_trait` 保证对象安全。*
3. **Cross-platform** — Same code runs natively and in WASM via conditional `cfg(target_arch)` dependencies.
   ***跨平台** —— 同一份代码通过条件依赖 `cfg(target_arch)` 既可原生运行，也可在 WASM 中运行。*
4. **Reuse run loops** — The ReAct loop in `ChatModelAgent` is reused by RouterSupervisor (tool calls as routing), avoiding duplicated orchestration logic.
   ***复用 run loop** —— `ChatModelAgent` 的 ReAct 循环被 RouterSupervisor 复用（tool call 即路由），避免编排逻辑重复实现。*
5. **No magic** — Each layer's behavior is explicit and traceable.
   ***无魔法** —— 每一层的行为都显式且可追溯。*

## Data Flow Example: A ReAct Agent · 数据流示例：ReAct Agent

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

## MultiAgent Coordination · MultiAgent 协调

Each MultiAgent pattern has a distinct coordination signal:
*每种 MultiAgent 模式都有各自不同的协调信号：*

| Pattern / 模式 | Coordinator / 协调者 | Signal / 信号 | Who Decides / 决策者 |
|---------|-------------|--------|-------------|
| Router/Supervisor / 路由式 | LLM (host) / LLM（host） | `tool_call` | Top-level LLM / 顶层 LLM |
| Host/Orchestrator / 编排式 | LLM (host) / LLM（host） | `tool_call` (parallel) / `tool_call`（并行） | Top-level LLM / 顶层 LLM |
| Conversational Handoff / 对话式 Handoff | None (rule-based) / 无（基于规则） | `event.action.transfer_to` | Sub-agent / sub-agent |
| Hybrid / 混合式 | LLM + handoff / LLM + handoff | both / 两者 | LLM picks, sub-agents can transfer / LLM 选定，sub-agent 可移交 |

See [MultiAgent Concepts](/concepts/multiagent) for details.
*详见 [MultiAgent Concepts](/concepts/multiagent) 文档。*
