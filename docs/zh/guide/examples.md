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

| [`a2a_interop_server`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_interop_server.rs) | 把 Synapse agent 作为 A2A v1.0 服务端架起来，让别的实现来调 |
| [`a2a_interop_client`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_interop_client.rs) | 调用远端 A2A 服务端（例如 Google ADK），打印它的卡片与答复 |

### 互操作探针（Python）

这两支**不是 cargo target** —— 它们要连一个活着的服务端跑，
并且需要 `pip install "google-adk[a2a]"`。

| 探针 | 演示内容 |
|---|---|
| [`a2a_sdk_probe.py`](https://github.com/iBLOC/synapse/blob/master/examples/a2a_sdk_probe.py) | 用 **Google 自己的 `a2a-sdk` 客户端**驱动服务端 —— 那是 ADK 随包发布的那一版 |
| [`adk_remote_probe.py`](https://github.com/iBLOC/synapse/blob/master/examples/adk_remote_probe.py) | 用 **ADK 自己的 `RemoteA2aAgent`** 驱动它 —— **ADK 用户真正看到的视角**；**不需要模型 key** |

```bash
cargo run --example a2a_interop_server -- 9099
python examples/adk_remote_probe.py
```

### ADK 作服务端（agent 模板）

[`adk_interop_agent/`](https://github.com/iBLOC/synapse/blob/master/examples/adk_interop_agent/)
是一个能直接跑的 ADK agent ⇒ ⇒ 反过来那条腿也能量：**Synapse 当客户端、
ADK 当服务端**。

```bash
pip install "google-adk[a2a]"
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent
cargo run --example a2a_interop_client -- http://127.0.0.1:8001/a2a/interop_agent
```

它**压根不调模型**就回 `ADK_OFFLINE_OK <你的原文>` —— **不需要 key、不需要网络**。
设上 `INTEROP_MODEL`（与 `GOOGLE_API_KEY`）就改成真调模型；key 一直待在
环境变量里。

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
4. Python 互操作探针不需要编译 —— 加进 *互操作探针* 那张表，并写清它需要什么（`google-adk[a2a]`）。

## 下一步

- [快速开始](/zh/guide/quick-start) — 带讲解的第一个 agent
- [实操指南](/zh/howto/simple-agent) — 手把手的实操教程
