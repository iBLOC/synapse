---
title: A2A Protocol
---

# A2A 协议

A2A（Agent-to-Agent）是 agent 之间跨进程、跨机器相互通信的协议。Synapse 同时实现了 A2A client 与 server。

## 为什么需要 A2A？

在多 agent 系统中，agent 经常需要：

  - 发送任务给专门化的远程 agent
  - 跨进程边界协调 workflow
  - 发现并调用 agent 的能力

A2A 为这些场景提供了标准协议（JSON over HTTP）。

## 架构

```
┌──────────────┐         HTTP         ┌──────────────┐
│  Synapse     │  ─────────────────→  │  A2A Server  │
│  A2A Client  │  (JSON-RPC-like)    │  (Other      │
│              │  ←─────────────────  │   process)   │
└──────────────┘                       └──────────────┘
```

## A2A 客户端

```rust
use synapse::a2a::A2AClient;

let client = A2AClient::new("http://localhost:8080").await?;
let response = client.send_message(message).await?;
```

## A2A 服务端

把 Synapse agent 托管为 A2A server：

```rust
use synapse::a2a::server::A2AServerBuilder;
use synapse::a2a::AgentCard;

let card = AgentCard::new("my-agent", "A helpful assistant", "http://0.0.0.0:8080", "1.0");
A2AServerBuilder::new(agent, card)
    .bind("0.0.0.0:8080".parse()?)
    .serve()
    .await?;
```

### 自定义 Task Store

默认使用 `InMemoryTaskStore`。如需持久化：

```rust
use synapse::a2a::store::FileTaskStore;

let store = FileTaskStore::new("/var/lib/my-agent/tasks").await?;
A2AServerBuilder::new(agent, card)
    .serve_with_store(store)
    .await?;
```

`FileTaskStore` 把 task 持久化为 JSON 文件，重启后仍然有效。


## 协议版本

Synapse 讲 **A2A v1.0**。版本在代理卡片的 `supportedInterfaces[].protocolVersion`
里公布，并按请求通过 `A2A-Version` 头协商。

> ⚠️ **对端必须发 `A2A-Version: 1.0`。** A2A v1.0 规范（§3.6.1/§3.6.2）
> 规定：**缺这个头的请求按版本 `0.3` 解释** —— 而 v1.0 服务端一律拒绝 `0.3`
>（`VERSION_NOT_SUPPORTED`）。Synapse 客户端会自动发这个头；
> **手写的对端必须自己发**。

## 扩展（Extensions）

代理可以在卡片里声明协议扩展，客户端可以按请求选用它们。

### 声明（服务端）

把扩展 URI 放进卡片的 `capabilities.extensions`。服务端会把它公布在代理卡片里，
**并且用同一份清单决定自己真能兑现哪些** —— 两边同源，所以卡片不可能
宣称一件代理做不到的事。

```rust
use synapse::a2a::AgentCard;

let mut card = AgentCard::new("my-agent", "A helpful assistant", "http://127.0.0.1:8080", "1.0.0");
card.capabilities.extensions = vec!["https://example.com/ext/v1".to_string()];
```

### 请求（客户端）

```rust
use synapse::a2a::client::A2AClient;

let client = A2AClient::with_extensions(
    "http://localhost:8080",
    vec!["https://example.com/ext/v1".to_string()],
)
.await?;
```

客户端会**同时发两种头名写法**，所以只认其中一种的对端也照样看得到请求：

* `A2A-Extensions` —— A2A v1.0 规范用的名字。
* `X-A2A-Extensions` —— Google ADK 仍在用的名字。

### 协商

服务端把请求里的 URI 与自己支持的取交集，并把结果报在事件 metadata 里：

* `activatedExtensions` —— 请求了**且**支持的。
* `unsupportedExtensions` —— 请求了但不支持的。

不支持的扩展是**报出来**而不是丢掉：客户端请求了一个扩展却收到沉默，
分不清是「不支持」还是「被忽略」—— 而这两种需要不同的反应。

协商结果挂在**事件** metadata 上，所以出现在流式路径（`SendStreamingMessage`）。
单发路径返回的是 `Task`，它的 metadata 归任务存储管，不归事件管。

## 任务生命周期

任务在服务端发出**终态**事件时才算结束 —— 也就是带终态（`completed`／
`failed`／`canceled`／`rejected`）的 `Task` 或 `StatusUpdate`。
`ArtifactUpdate` 是**内容**不是状态：最终答复以 artifact 的形式到达，
而任务在那一刻**仍然开着**。

synapse 的执行器自己发这个终态事件。代理的流如果没给出最终答复，
任务标成**失败**而不是完成 —— 报「完成」却交不出答复，就是**失败长得像成功**。

> ⚠️ **这不是讲究。** Google ADK 的 `RemoteA2aAgent` 把停在 `submitted`／
> `working` 的任务读成**流被截断**，并抛错而不是把答复交回去
>（`google/adk/a2a/agent/_remote_a2a_agent.py` 里的
> `_UNFINISHED_TASK_STATES`）⇒ ⇒ **从不收尾的服务端会把每一个 ADK 客户端
> 都变成失败**。

服务端也会公布 `capabilities.streaming: true` —— 它确实提供
`SendStreamingMessage`；卡片若写「不支持流式」，客户端就会退回单发路径，
而 A2A v1.0 的单发路径返回的是从 store 重载的 Task，
事件 metadata（扩展协商的结果）在那条路上整个丢掉。

## 与其它实现的互操作

仓里有两支探针：

* `examples/a2a_sdk_probe.py` —— 用 **Google 自己的 `a2a-sdk` 客户端**
  （ADK 随包发布的那一版）驱动一个活着的 synapse 服务端。它量四件事：
  卡片解析、客户端默认路径、服务端存下来的任务，
  以及一个去掉 `A2A-Version` 头的负控。
* `examples/adk_remote_probe.py` —— 用 **ADK 自己的 `RemoteA2aAgent`**
  驱动它，也就是 **ADK 用户真正看到的那个视角**。**不需要模型 key**：
  `RemoteA2aAgent` 只做转发，**它自己不调模型**。

```bash
cargo run --example a2a_interop_server -- 9099
pip install "google-adk[a2a]"
python examples/a2a_sdk_probe.py
python examples/adk_remote_probe.py
```

`adk_remote_probe.py` 还带一个**因果负控**：进程内起一个假对端，
**复刻修前的行为**（答复以 artifact 到达，但任务不到终态）。
ADK 于是报出

    A2A response stream ended before task t-pre-fix reached a terminal state.

—— **答复其实到了**，ADK 仍然把整轮判成失败。
把终态事件放回去，同一支探针读到的是 `SYNAPSE_REPLIED_OK` 且没有错误。
同一个客户端、同一条对端代码路径，**唯一变量就是终态事件**。

### ADK 作服务端

`examples/adk_interop_agent/` 是一个能直接跑的 ADK agent ⇒ ⇒ 反过来的
那条腿（**Synapse 当客户端、ADK 当服务端**）也能验：

```bash
pip install "google-adk[a2a]"

# 不需要 key 的那一档：不调模型、不联网
adk api_server --a2a --port 8001 --host 127.0.0.1 examples/adk_interop_agent

# 然后从 Synapse 这一侧打过去
cargo run --example a2a_interop_client -- http://127.0.0.1:8001/a2a/interop_agent
```

答复是 `ADK_OFFLINE_OK <你的原文大写>` —— 真调模型时是
`ADK_MODEL_OK <你的原文大写>`。⇒ ⇒ **两个前缀故意不同**：离线的答复
**不能长得像**模型的答复。

要真调模型，设两个环境变量即可：

```bash
export GOOGLE_API_KEY=...        # 或 GEMINI_API_KEY
export INTEROP_MODEL=gemini-3.5-flash
```

**key 绝不进 agent 文件** —— ADK 自己的 `Gemini` 类是从环境里拿 key 的
（先 `GOOGLE_API_KEY`，后 `GEMINI_API_KEY`）⇒ ⇒ 拿 key 的是**服务端进程**，
不是 agent。而且用的是 `Gemini` 而**不是** `LiteLlm`：Google AI Studio 的 key
只在**原生 `google-genai` 那条路**上认（OpenAI 兼容面逐字回 `Invalid Auth key`）。

> ⚠️ **卡片里的 `url` 必须是真实挂载点** —— ADK 把 `agent.json` 的 `url`
> **原样**抄进卡片（`supportedInterfaces[0].url`），**不改写** ⇒ ⇒ JSON-RPC
> 的路由是 `/a2a/<agent_name>` ⇒ ⇒ 卡片写 `http://127.0.0.1:8001/` 的话，
> 照卡片办事的客户端就打到错地方了（`POST /` 回 404、`GET /` 回 405）⇒ ⇒
> 换端口时两处要一起改。

### 答复住在哪

任务的答复住在 **`task.artifacts`** ⇒ ⇒ ADK 自己的转换器读的就是它
（`convert_a2a_task_to_event` 把**每个** artifact 的 parts 摊平），
Synapse 的客户端也读它。

`task.history` **不是**答复 —— 对端可能把**调用方自己**那一轮放进那里
（Synapse 自己的服务端就这么干）⇒ ⇒ 「history 最后一条」可能就是调用方
自己的原话，被当成 agent 的答复交回去了。

`status.message` 只在**边界态**里当答复（`input-required`／`auth-required`，
或者「完成了但没有 artifact」）⇒ ⇒ **失败态**里它是**错误**：ADK 是给事件
盖上 error code ＋ error message，而不是把那段文字当内容交回去。Synapse 的
客户端把**失败／取消／拒绝**的任务一律报成失败 —— 把「401 UNAUTHENTICATED」
当成一条完好的答复交出去，就是**失败长得像成功**。

> ⚠️ ADK 把答复**同时**放在 working 态的 status message **和** artifact 里
> ⇒ ⇒ 两处都读的客户端会把答复显示两遍 ⇒ ⇒ Synapse 的流式路径**只**读 artifact。
### 流式路径保证什么

流式路径另有**三条**规矩 ⇒ ⇒ 每一条都是一处「看着没问题、其实有缺陷」的地方：

* **答复是整个 artifact，不是它的最后一块** —— 对端可以分块流式送答复：
  第一个更新**建** artifact（`append: false`），之后的**续**（`append: true`），
  最后一块（`lastChunk: true`）收尾 ⇒ ⇒ Final 带的是**累加出来的全文**。
* **流结束了 ≠ 任务结束了** —— 任务只有在收到**终态**时才算结束；流自己断了
  （对端崩了／网断了）就是**被截断** ⇒ ⇒ 客户端按失败报出去（与 ADK 自己的
  客户端同口径）。
* **答复也可能在 status message 里** —— 有些对端**只**放那儿 ⇒ ⇒ 当没有任何
  artifact 带过文本时，status message 就是答复。

> ⚠️ ADK 把答复**同时**放在 working 态的 status message **和** artifact 里
> ⇒ ⇒ 两处都读的客户端会把答复显示两遍 ⇒ ⇒ Synapse 读 artifact，
> status message 只当兜底。

### 对端就在这台机器上

在 `127.0.0.1` 上的对端**不许**走系统代理：reqwest 会从操作系统那里抓代理
（Windows 上是注册表的 `ProxyServer`），而系统自己那份绕过名单（注册表的
`ProxyOverride`，名单里恰恰写着 `localhost` 与 `127.*`）**对回环不生效**
⇒ ⇒ 于是回环对端被丢给代理 ⇒ ⇒ 代理够不到本机自己的回环口 ⇒ ⇒ 回
**`502 Bad Gateway`**（空 body）。

⇒ ⇒ 也**别指望**那份名单兜底：hyper-util 只在**没有** `NO_PROXY`／`no_proxy`
环境变量时才去读注册表的 `ProxyOverride` ⇒ ⇒ 一个环境变量就把那份名单
**整份顶掉**。

⇒ ⇒ Synapse 按 URL 判定：`synapse::http_client::client_for` 对**回环对端
绕过代理**，对**远端对端保留代理**（远端多半真需要它）⇒ ⇒ 而所有可能打到
本机对端的产品路径都走这**同一个模块** —— A2A 客户端、`v1_compat` 客户端、
端脑注册、服务注册、lobe 的 OpenAI 兼容 LLM 客户端、决策存储、SSE 传输。
### ADK 那个专属扩展

ADK 自带一个扩展：`https://google.github.io/adk-docs/a2a/a2a-extension/`。
它的含义是「**用 ADK 的新版 agent executor**」—— 修的是 ADK 自己 executor 里的
消息重复／输出误判／子代理丢数。synapse 没有 ADK executor，
所以**不宣称支持**这个扩展。诚实的回答是把它报成不支持，
而这正是协商机制已经在做的事：

    unsupportedExtensions: ["https://google.github.io/adk-docs/a2a/a2a-extension/"]

ADK 于是走它的 legacy 路径 —— 而那本来就是普通 A2A 对端该走的路。
## 任务存储 trait

实现 `TaskStore` trait 即可自定义持久化：

```rust
use synapse::a2a::store::TaskStore;

struct MyCustomStore { ... }

#[async_trait]
impl TaskStore for MyCustomStore {
    async fn create(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn update(&self, task: Task) -> Result<TaskVersion, A2AError> { ... }
    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> { ... }
    async fn list(&self, req: &ListTasksRequest) -> Result<ListTasksResponse, A2AError> { ... }
}
```

## 相关阅读

  - [WASM Sandbox](/zh/concepts/wasm) —— WASM 上下文中的 A2A
