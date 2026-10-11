---
title: Introduction
---

# 简介

Synapse 是一个基于 Rust 的智能 agent 开发框架，与 LangChain、Eino 同类，为 LLM 应用提供核心构件：agent、tool、RAG、多 agent 协作、MCP、A2A、ACP 与 WASM 沙箱。

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

  - A2A 协议 —— 一等公民的跨进程 agent 通信
  - ACP 客户端 —— 把外部编码 agent（opencode、claude-code、codex）接成普通的 Synapse 流
  - WASM 隔离 —— agent 跑在 `wasi` 沙箱里（PFC runtime）
  - MultiAgent 四种模式全覆盖 —— Router、Orchestrator、Handoff、Hybrid
  - 类型安全的 Rust API —— 无 Python runtime 开销，原生异步
  - 292 个测试 —— 完整的单元 + 集成测试覆盖，全部 mock

## 什么时候用 Synapse？

当你需要以下能力时，选 Synapse：

  - 带 tool 调用、ReAct 或图式编排的 LLM agent
  - RAG pipeline（解析 → 分块 → embedding → 检索）
  - 多 agent 系统（路由、任务分解、handoff、混合）
  - MCP tool 集成（stdio 或 HTTP）
  - 跨进程 agent 通信（A2A）
  - 通过 Agent Client Protocol 驱动外部编码 agent（opencode、claude-code-acp、codex-acp）
  - 部署到 WASM 沙箱以获得隔离

## 项目状态

  - 版本：0.1.0
  - 许可证：Apache 2.0
  - 测试：292 个全部通过
  - 可上生产的组件：核心 agent/RAG/MCP/A2A/MultiAgent
  - API 稳定：是（遵守 semver）

## 下一步

- **[快速开始](/zh/guide/quick-start)** — 5 分钟构建你的第一个 agent
- **[架构](/zh/concepts/architecture)** — 理解框架架构
- **[实操指南](/zh/howto/simple-agent)** — 手把手的实操教程
- **[API 参考](/zh/reference/agent-trait)** — 详细 API 文档

## 相关项目

  - Synapse（本项目）—— agent 开发工具箱
  - Neuron —— LLM 推理服务 runtime
  - PFC —— 多 agent 编排器 + WASM 沙箱 runtime
