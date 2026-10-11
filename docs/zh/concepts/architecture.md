---
title: Architecture
---

# 架构

Synapse 按分层模块组织，每一层都构建在其下层的抽象之上。

## 分层总览

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

`acp` 和 `ziti` 是**可选 feature 门控**（`--features acp` / `--features ziti`）——默认构建不编译它们。`acp` 还仅在非 wasm 目标可用。

## 核心设计原则

1. **基于 trait 的组合** —— 每个主要抽象都是 trait，用户可以插入自定义实现。
2. **默认异步** —— 所有 I/O 都是 `async`，用 `async_trait` 保证对象安全。
3. **跨平台** —— 同一份代码通过条件依赖 `cfg(target_arch)` 既可原生运行，也可在 WASM 中运行。
4. **复用 run loop** —— `ChatModelAgent` 的 ReAct 循环被 RouterSupervisor 复用（tool call 即路由），避免编排逻辑重复实现。
5. **无魔法** —— 每一层的行为都显式且可追溯。

## 数据流示例：ReAct Agent

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

## MultiAgent 协调

每种 MultiAgent 模式都有各自不同的协调信号：

| 模式 | 协调者 | 信号 | 决策者 |
|---------|-------------|--------|-------------|
| 路由式 | LLM (host) / LLM（host） | `tool_call` | 顶层 LLM |
| 编排式 | LLM (host) / LLM（host） | `tool_call`（并行） | 顶层 LLM |
| 对话式 Handoff | 无（基于规则） | `event.action.transfer_to` | Sub-agent / sub-agent |
| 混合式 | LLM + handoff / LLM + handoff | 两者 | LLM 选定，sub-agent 可移交 |

详见 [MultiAgent Concepts](/zh/concepts/multiagent) 文档。
