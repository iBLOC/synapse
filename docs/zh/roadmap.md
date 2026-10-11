---
title: Roadmap
---

# 路线图

Synapse 的未来规划方向。

## 当前（0.1.x）

  - ✅ 核心 Agent trait
  - ✅ Tool 框架（BaseTool、InvokableTool、EnhancedInvokableTool）
  - ✅ RAG（分块、embedding、向量库、检索器）
  - ✅ A2A 协议
  - ✅ 跨平台（native + WASM）
  - ✅ OpenAI + Anthropic LLM 支持
  - ✅ Document 统一 + JSON 序列化
  - ✅ Apache 2.0 许可
  - ✅ 205 个测试

## 短期（0.2.x）

  - 更多向量库后端（LanceDB、Qdrant、pgvector）
  - 所有 chat model 支持流式响应
  - 长时运行 agent 的完整 resume/checkpoint
  - 内置 A2A server（基于 axum）
  - 更多文档解析器（PDF、DOCX 等）
  - 感知 HTML 结构的 chunker
  - 感知代码结构的 chunker（按函数/类边界）

## 中期（0.3.x+）

  - Agent 可观测性（集成 OpenTelemetry）
  - Prompt 模板 DSL
  - Agent 记忆管理（短期 + 长期）
  - 多模态支持（视觉、音频）
  - 分布式 agent 执行
  - 端脑（Telencephalon）集成（把 agent 部署到端脑）
  - Neuron LLM runtime 集成

## 长期

  - 可视化 agent 构建器（no-code）
  - Agent 市场 / 注册中心
  - 性能基准测试
  - 生产部署指南

## 参与贡献

想参与路线图的规划？在 GitHub 上开 issue，或查阅 [CONTRIBUTING.md](https://github.com/ibloc/synapse/blob/master/CONTRIBUTING.md)。
