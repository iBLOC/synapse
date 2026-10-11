---
title: Comparison with LangChain / Eino / BeeAI
---

# 与 LangChain / Eino / BeeAI 的对比

Synapse 的定位是 LangChain、Eino 和 BeeAI 的 Rust 原生替代方案，在跨平台部署、A2A 协议和 WASM 沙箱方面具有独特优势。

## 功能矩阵

| 特性 | Synapse | LangChain | Eino | BeeAI |
|---------|---------|-----------|------|-------|
| 语言 | Rust | Python/JS | Go | Python/TypeScript |
| 治理 | Apache 2.0 | MIT | Apache 2.0 | Linux Foundation |
| 可运行组合 | ✅ | ✅ | ✅ | ✅ |
| 图工作流 | ✅ | ✅ (LangGraph) | ✅ | ✅ |
| ReAct agent（多轮） | ✅ | ✅ | ✅ | ✅ |
| 流式 | 部分 | 完整 | ✅ | ✅ |
| 工具中间件 | ✅ | ✅ | ✅ | 事件驱动 |
| 原生异步 | ✅ | 部分 | ✅ | ✅ |
| 类型安全 | ✅ | ❌ (Python) | ✅ | TS 部分 |
| 性能 | ✅ | ⚠️ | ✅ | ⚠️ |
| RAG（分块） | 4 种 splitter | 众多 | ✅ | 外部 |
| RAG 向量存储 | 仅 InMemory | ✅ 100+ | 较少 | 外部 |
| Embedding providers / Embedding provider | ⚠️ 3 (OpenAI/Anthropic/Azure) | 众多 | 众多 | ✅ 10+ |
| LLM providers / LLM provider | ⚠️ 3 | 众多 | 众多 | ✅ 10+ |
| 多 Agent | 4 种模式 | ✅ | ✅ | ✅ |
| MCP | ✅ stdio + HTTP | ✅ | ❌ | 原生 |
| A2A 协议 | ✅ | ❌ | ❌ | 原生 |
| WASM 部署 | 原生 | ❌ | ❌ | ❌ |
| 跨平台（native + WASM） | ✅ | ❌ | ❌ | ❌ |
| 内存管理 | ❌ (0.3.x) | ✅ | ⚠️ | 内置 |
| OpenTelemetry 可观测性 | ✅ Span/Trace (0.4) | ✅ | ⚠️ | 原生 |
| 强类型 Lambda | ✅ (0.4) | ❌ | ✅ | ❌ |
| LCEL 风格 pipe | ✅ (0.4) | ✅ | ❌ | ❌ |
| 声明式 YAML 编排 | ❌ | 部分 | ⚠️ | ✅ |
| 内置缓存 | ❌ | ✅ | ⚠️ | ✅ |
| 约束执行 | ❌ | ❌ | ❌ | ✅ |
| 测试 | ~310 | N/A | N/A | N/A |

## Synapse 的独特优势

   1. A2A protocol —— 一等公民的跨进程 agent 通信。
   2. WASM 沙箱 —— 把 agent 部署到沙箱环境。
   3. MCP HTTP —— 同时支持 stdio 和 HTTP 两种 transport。
   4. MultiAgent 覆盖 —— 用一套一致的 API 覆盖全部 4 种模式。
   5. 类型安全 —— Rust 类型系统在编译期就能抓住 agent 接线错误。
   6. 强类型 Lambda —— Eino 风格的类型安全组合（0.4 新增）。
   7. LCEL 风格 pipe —— LangChain 风格的可链式表达式（0.4 新增）。
   8. Span/Trace 可观测性 —— BeeAI 风格的 tracing，exporter 可插拔（0.4 新增）。
   9. Apache 2.0 —— 开源许可证。

## Synapse 的局限

   1. 生态 —— LangChain 有 100+ 集成；Synapse 只有核心组件。
   2. 向量存储 —— 目前仅支持内存。
   3. 流式 —— 部分支持。
   4. 内存管理 —— 尚未内置（计划于 0.3.x）。
   5. OpenTelemetry exporter —— OTel 需要自定义 exporter（计划中）。
   6. 社区 —— 新项目，社区规模较小。

## 0.4 新特性

  - Typed Lambda —— Eino 风格的强类型 Lambda 组合。
  - LCEL Pipe —— LangChain 风格的可链式表达式。
  - 可观测性 —— BeeAI 风格的 Span/Trace，带 exporter。
  - 框架对齐 —— Synapse 如何对齐 Eino、BeeAI、LangChain。

## 何时选择 Synapse

  Rust 性能与类型安全
  用 A2A 协议连接跨进程 agent
  把 agent 部署进 WASM 沙箱
  Apache 2.0 开源
  正在构建多 agent 系统

## 何时选择 LangChain

  要最大生态（100+ 集成）
  Python 或 JS 生态
  成熟、久经生产考验的技术栈

## 何时选择 Eino

  偏好 Go（云原生生态）
  需要深度字节跳动集成

## 何时选择 BeeAI

  Linux Foundation 治理，厂商中立的开源
  Python 或 TypeScript，功能完全对齐
  内置内存管理（无界 / 摘要式 / token 窗口）
  声明式的 YAML agent 编排
  生产级约束执行 + 缓存
  开箱即用的原生 OpenTelemetry 可观测性
  MCP 与 A2A 互操作为一等公民
