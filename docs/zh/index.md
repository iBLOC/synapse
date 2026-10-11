---
title: Synapse 是什么？
description: 基于 Rust 的智能 agent 开发框架
layout: home

hero:
  name: Synapse
  text: Rust Agent 框架
  tagline: ReAct · RAG · MultiAgent · MCP · A2A · ACP · WASM
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/quick-start
    - theme: alt
      text: GitHub
      link: https://github.com/iBLOC/synapse

features:
  - title: 🚀 生产可用的核心
    details: Runnable、Graph、Chain、ReAct Agent。292 个测试全部通过。Apache 2.0 协议。
  - title: 🤖 4 种 MultiAgent 模式
    details: Router、Orchestrator、Handoff、Hybrid。支持嵌套组合，LLM 驱动与规则式并存。
  - title: 📚 开箱即用的 RAG
    details: 4 种 chunker（Character/Recursive/Markdown/Token）、OpenAI embedding、内存向量库。
  - title: 🔌 MCP + A2A + ACP
    details: Model Context Protocol（stdio + HTTP + Server）。Agent-to-Agent 协议。ACP 客户端（驱动 opencode / claude-code / codex）。
  - title: 📝 Session 事件日志
    details: 只追加的事件持久化。FileEventStore 采用 JSON Lines 格式。
  - title: 🌊 Waterfall 事件
    details: 5 种分发模式（emit/parallel/serial/bail/waterfall）。拦截器模式。
  - title: 🔄 LLM 重试策略
    details: 指数退避 + 抖动。可配置可重试的错误码。
  - title: 🏷️ Tool 命名空间
    details: 作用域化的 skill 注册。MCP 风格的限定名（mcp__server__tool）。
  - title: 🌐 WASM 沙箱就绪
    details: 跨平台 HttpClient。agent 跑在 wasi 沙箱里（PFC runtime）。
  - title: 🛡️ 类型安全的 Rust
    details: 原生异步，无 Python runtime，类型安全的 agent 组合。
---

## 为什么选择 Synapse？

| 能力 | Synapse | LangChain | Eino |
|------------|---------|-----------|------|
| Runnable/Graph/Chain | ✅ | ✅ | ✅ |
| 多轮 ReAct Agent | ✅ | ✅ | ✅ |
| MultiAgent 模式 | ✅（4 种模式） | ✅ | ✅ |
| 带 chunking 的 RAG | ✅（4 种 splitter） | ✅ | ✅ |
| MCP（模型上下文协议） | ✅ | ✅ | ❌ |
| A2A（Agent 到 Agent） | ✅ | ❌ | ❌ |
| WASM 沙箱执行 | ✅ | ❌ | ❌ |
| 跨平台（Native + WASM） | ✅ | ❌ | ❌ |
| 纯 Rust | ✅ | ❌ | ❌ |

Synapse 的独特优势：

- A2A 协议 —— 一等公民的跨进程 agent 通信，带持久化 task 存储
- ACP 客户端 —— 把外部编码 agent（opencode、claude-code、codex）接成普通的 Synapse 流
- WASM 隔离 —— agent 跑在 `wasi` 沙箱里（PFC runtime）
- MultiAgent 四种模式全覆盖 —— Router、Orchestrator、Handoff、Hybrid
- MCP Server —— 用 `SimpleMcpServer` 构建自定义 MCP server
- 类型安全的 Rust API —— 无 Python runtime 开销，原生异步
- 292 个测试 —— 完整的单元 + 集成测试覆盖，全部 mock

## 下一步

- **[快速开始](/zh/guide/quick-start)** — 5 分钟构建你的第一个 agent
- **[架构](/zh/concepts/architecture)** — 理解框架设计
- **[实操指南](/zh/howto/simple-agent)** — 手把手的实操教程
- **[API 参考](/zh/reference/agent-trait)** — 详细 API 文档

## 相关项目

- **Synapse**（本项目）—— agent 开发工具箱
- **Neuron** —— LLM 推理服务 runtime
- **PFC** —— 多 agent 编排器 + WASM 沙箱 runtime
