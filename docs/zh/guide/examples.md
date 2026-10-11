---
title: 示例
---

# 示例

可直接运行的示例在仓库的 [`examples/`](https://github.com/iBLOC/synapse/tree/master/examples) 目录。所有示例都能通过 `cargo build --examples` 编译；运行需要一个 OpenAI 兼容的 endpoint（把源码里的 `sk-...` 占位符换成你的 key）。

| 示例 | 演示内容 |
|---|---|
| [`simple_agent`](https://github.com/iBLOC/synapse/blob/master/examples/simple_agent.rs) | 最小 agent：`OpenAIModel` + `GraphReActAgent`，一个 prompt 进、一个回答出 |
| [`agent_with_tools`](https://github.com/iBLOC/synapse/blob/master/examples/agent_with_tools.rs) | 自定义 tool（`BaseTool` + `InvokableTool`）装进 `ToolsNode` 挂到 ReAct 循环 |
| [`react_agent`](https://github.com/iBLOC/synapse/blob/master/examples/react_agent.rs) | 多轮对话 —— 由调用方持有跨轮次的消息历史 |
| [`rag_pipeline`](https://github.com/iBLOC/synapse/blob/master/examples/rag_pipeline.rs) | 分块 → embedding → 索引 → 检索，使用内存向量库 |

## 运行

```bash
git clone https://github.com/iBLOC/synapse
cd synapse
cargo run --example simple_agent
```

不想用托管 API 的话，用 `.with_base_url("http://localhost:port/v1")` 把模型指向本地 OpenAI 兼容服务即可。

## 新增示例

1. 创建 `examples/<your_example>.rs` —— 示例与其他 target 一样参与编译，`cargo build --examples` 会在 CI 里把关。
2. 文件顶部的 doc comment 说明演示内容和运行方式。
3. 在 `docs/guide/examples.md` 和 `docs/zh/guide/examples.md` 的表格里各加一行。

## 下一步

- [快速开始](/zh/guide/quick-start) — 带讲解的第一个 agent
- [实操指南](/zh/howto/simple-agent) — 手把手的实操教程
