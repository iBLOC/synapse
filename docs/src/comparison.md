# Comparison with LangChain / Eino

Synapse is positioned as a Rust-native alternative to LangChain and
Eino, with unique strengths in cross-platform deployment and A2A
protocols.

## Feature Matrix

| Feature | Synapse | LangChain | Eino |
|---------|---------|-----------|------|
| Language | Rust | Python/JS | Go |
| Runnable composition | ✅ | ✅ | ✅ |
| Graph workflows | ✅ | ✅ (LangGraph) | ✅ |
| ReAct agent (multi-turn) | ✅ | ✅ | ✅ |
| Streaming | ⚠️ Partial | ✅ Full | ✅ |
| Tool middleware | ✅ | ✅ | ✅ |
| Async native | ✅ | ⚠️ Partial | ✅ |
| Type safety | ✅ | ❌ (Python) | ✅ |
| Performance | ✅ | ⚠️ | ✅ |
| RAG (chunking) | ✅ 4 splitters | ✅ Many | ✅ |
| RAG vector stores | ⚠️ InMemory only | ✅ 100+ | ⚠️ Few |
| Embedding providers | ⚠️ OpenAI only | ✅ Many | ✅ Many |
| Multi-agent | ✅ 4 patterns | ✅ | ✅ |
| MCP | ✅ stdio + HTTP | ✅ | ❌ |
| A2A protocol | ✅ | ❌ | ❌ |
| WASM deployment | ✅ Native | ❌ | ❌ |
| Cross-platform (native + WASM) | ✅ | ❌ | ❌ |
| Tests | 157 | N/A | N/A |

## Synapse's Unique Strengths

1. **A2A protocol** — First-class cross-process agent communication.
   No equivalent in LangChain or Eino.
2. **WASM sandbox** — Deploy agents to sandboxed environments.
   `WasiHttpClient` makes this seamless.
3. **MCP HTTP** — Support both stdio and HTTP transports.
4. **MultiAgent coverage** — All 4 patterns (Router, Orchestrator,
   Handoff, Hybrid) in one consistent API.
5. **Type safety** — Rust's type system catches agent wiring errors
   at compile time.

## Synapse's Limitations

1. **Ecosystem** — LangChain has 100+ integrations; Synapse has core
   components only.
2. **Vector stores** — Currently in-memory only.
3. **Streaming** — Partial support (the trait exists, but only
   `OpenAIModel` has full streaming).
4. **Community** — New project, smaller community.

## When to Choose Synapse

✅ You want **Rust performance and type safety**
✅ You need **A2A protocol** for cross-process agents
✅ You need to deploy agents in **WASM sandboxes**
✅ You want **Apache 2.0** open source (vs LangChain's MIT)
✅ You're building a **multi-agent system**

## When to Choose LangChain

✅ You need **maximum ecosystem** (100+ integrations)
✅ You want **Python or JS** ecosystem
✅ You need **mature, battle-tested** production stack

## When to Choose Eino

✅ You prefer **Go** (Cloud Native ecosystem)
✅ You need **deep ByteDance integration**
