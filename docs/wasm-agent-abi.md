# WASM Agent ABI Reference · WASM Agent ABI 参考

> The contract between a Synapse-compiled agent (`my_agent.wasm`)
> and the supervisor (Prefrontal, or any layer-3 host) that loads
> and runs it. Synapse guarantees compatibility with this ABI
> across minor (0.x.y) versions; breaking changes require a
> major version bump.
> *Synapse 编译出的 agent（`my_agent.wasm`）与加载并运行它的 supervisor（Prefrontal 或任意 layer-3 宿主）之间的契约。Synapse 保证此 ABI 在 minor（0.x.y）版本之间保持兼容；破坏性变更需要 bump major 版本。*

## ABI version · ABI 版本

Current: **1**
*当前版本：**1**。*

Each WASM image MUST export a function:
*每个 WASM image 都必须（MUST）导出一个函数：*

```
synapse_abi_version() -> i32
```

returning the integer `1`. Supervisors reject images whose
returned version differs from the supported range.
*即返回整数 `1`。对返回版本不在支持范围内的 image，supervisor 一律拒绝。*

## Memory model · 内存模型

The WASM module owns a linear memory. All strings are passed as
`(ptr: i32, len: i32)` pointing into that memory. Strings are
always UTF-8.
*WASM module 拥有一块线性内存。所有字符串都以 `(ptr: i32, len: i32)` 指向该内存的方式传递，且始终为 UTF-8。*

The host pre-allocates a **stream response buffer** (default 64 KiB,
configurable per agent) per call to `stream_run`. The agent writes
chunks of `AgentStreamEvent` (CBOR or JSON, see §5) into this
buffer; the host polls and forwards.
*宿主在每次 `stream_run` 调用时预分配一个**流式响应缓冲区**（默认 64 KiB，可按 agent 配置）。agent 把一段段 `AgentStreamEvent`（CBOR 或 JSON，见 §5）写入该缓冲区；宿主轮询并转发。*

## Exports (agent → host) · 导出（agent → host）

These are the functions every Synapse agent MUST export.
*这些是每个 Synapse agent 必须导出的函数。*

### `agent_card`

```
agent_card(out_ptr: i32) -> i32
```

Writes an `AgentCard` JSON document into linear memory at `out_ptr`.
Returns the byte length written. Returns `-1` on error.
*把一份 `AgentCard` JSON 文档写入 `out_ptr` 处的线性内存，返回写入的字节长度；出错时返回 `-1`。*

```json
{
  "name": "weather-agent",
  "description": "Looks up weather forecasts.",
  "version": "1.2.0",
  "skills": [
    {"id": "get_weather", "description": "...", "input_schema": {...}}
  ],
  "default_input_modes": ["text/plain"],
  "default_output_modes": ["text/plain"]
}
```

Conforms to the v1.0 A2A `AgentCard` schema
<https://a2a-protocol.org/latest/specification/#agent-card>.
*符合 v1.0 A2A `AgentCard` schema（<https://a2a-protocol.org/latest/specification/#agent-card>）。*

### `stream_run`

```
stream_run(
  input_ptr: i32,           // JSON encoded AgentInput
  input_len: i32,
  out_handle_ptr: i32       // host writes a u32 stream handle here
) -> i32                     // 0 on success, negative on error
```

Begins a streaming execution. The agent writes a sequence of
`AgentStreamEvent`s into the per-handle response buffer. The host
drains the buffer until the agent writes a terminal `Final`/`Error`
event, then closes the stream handle.
*开始一次流式执行。agent 把一串 `AgentStreamEvent` 写入该 handle 对应的响应缓冲区；宿主持续排空缓冲区，直到 agent 写入终止性的 `Final`/`Error` 事件，然后关闭流 handle。*

### `cancel_run`

```
cancel_run(handle: u32) -> i32
```

Asks the agent to abort the stream identified by `handle`. The
agent SHOULD emit a final `Error { error: AgentError::Interrupted }`
event and close its end of the buffer.
*请求 agent 中止 `handle` 标识的流。agent 应当（SHOULD）发出一个最终的 `Error { error: AgentError::Interrupted }` 事件，并关闭自己一侧的缓冲区。*

### `query_task`

```
query_task(
  task_id_ptr: i32,
  task_id_len: i32,
  out_ptr: i32
) -> i32  // returns length of `Task` JSON written to out_ptr
```

Returns the current A2A `Task` state for the given `task_id`, or
`-1` if the task is unknown.
*返回给定 `task_id` 当前的 A2A `Task` 状态；任务未知时返回 `-1`。*

### `subscribe_task`

```
subscribe_task(
  task_id_ptr: i32,
  task_id_len: i32,
  out_handle_ptr: i32      // host writes a u32 stream handle here
) -> i32
```

Like `stream_run` but for an existing task. Returns a stream handle
that yields `StatusUpdateEvent` / `ArtifactUpdateEvent`s.
*与 `stream_run` 类似，但针对已存在的 task。返回一个流 handle，产出 `StatusUpdateEvent` / `ArtifactUpdateEvent`。*

## Imports (host → agent) · 导入（host → agent）

The host MUST provide the following imports.
*宿主必须（MUST）提供以下 import。*

### `wasi:http/outgoing-handler` (and friends) · 及相关 import

Standard WASI HTTP for outbound HTTPS calls. Used by the agent
to reach LLM providers. See <https://github.com/WebAssembly/wasi-http>.
*标准的 WASI HTTP，用于出站 HTTPS 调用；agent 借此访问 LLM provider。见 <https://github.com/WebAssembly/wasi-http>。*

### `synapse:host/get_config`

```
get_config(out_ptr: i32, out_cap: i32) -> i32  // returns length written
```

Returns the agent's runtime configuration as a JSON document:
*以 JSON 文档返回 agent 的运行时配置：*

```json
{
  "llm_base_url": "https://api.openai.com/v1",
  "llm_api_key":  "sk-...",
  "llm_model":    "gpt-4o",
  "llm_extra":    { "organization": "..." },
  "a2a_peers": [
    {"name": "router", "url": "https://router.example.com"}
  ]
}
```

The agent calls this **once at startup** (and re-reads after every
`cancel_run` to pick up rotation). The host controls freshness.
*agent **仅在启动时**调用一次（并在每次 `cancel_run` 之后重新读取，以获取轮换后的配置）。新鲜度由宿主控制。*

### `synapse:host/a2a_call`

```
a2a_call(
  peer_name_ptr: i32, peer_name_len: i32,
  msg_ptr: i32, msg_len: i32,
  out_ptr: i32, out_cap: i32,
  out_len_ptr: i32
) -> i32  // status code: 0 ok, negative on error
```

Sends an A2A `SendMessageRequest` to the named peer (which must
appear in `a2a_peers` config). The host performs the actual HTTP
call with native `a2a-rs` client SDK, since WASM cannot drive
`reqwest`. Returns the `SendMessageResponse` JSON in `out_ptr`
and the length in `out_len_ptr`.
*向指定 peer 发送 A2A `SendMessageRequest`（该 peer 必须出现在 `a2a_peers` 配置中）。实际的 HTTP 调用由宿主用原生 `a2a-rs` client SDK 完成，因为 WASM 无法驱动 `reqwest`。`SendMessageResponse` JSON 写入 `out_ptr`，长度写入 `out_len_ptr`。*

This is a synchronous call from the agent's perspective. For long
peers the agent SHOULD dispatch it on a `tokio::spawn` and await
its result.
*从 agent 视角看这是同步调用。对响应较慢的 peer，agent 应当（SHOULD）把它 `tokio::spawn` 出去并 await 结果。*

## Event payload format · 事件负载格式

Events written by the agent into the stream response buffer use
the same `AgentStreamEvent` schema as the in-process Synapse
streaming API:
*agent 写入流式响应缓冲区的事件，与进程内 Synapse streaming API 使用相同的 `AgentStreamEvent` schema：*

```json
{
  "event": "TokenDelta",
  "agent": "weather-agent",
  "delta": "The current"
}
```

```json
{
  "event": "ToolStart",
  "agent": "weather-agent",
  "name": "get_weather",
  "args": "{\"city\":\"SF\"}"
}
```

```json
{
  "event": "Final",
  "agent": "weather-agent",
  "message": {"role": "assistant", "content": [{"type": "text", "text": "..."}]}
}
```

See `docs/superpowers/specs/2026-06-30-streaming-design.md §4.2`
for the full variant list. The wire encoding is one JSON object per
event, **newline-delimited** (NDJSON) so a partial write at the end
of the buffer can be retried on the next poll cycle.
*完整 variant 列表见 `docs/superpowers/specs/2026-06-30-streaming-design.md §4.2`。线上编码为每个事件一个 JSON 对象，以**换行分隔**（NDJSON），这样缓冲区末尾的不完整写入可以在下一轮轮询时重试。*

## Stability policy · 稳定性策略

- **Within a single `abi_version`**: the host MUST accept any agent
  that exports the same set of symbols with the same signatures.
  *同一 `abi_version` 之内：宿主必须接受任何导出相同符号集合且签名一致的 agent。*
- **Across `abi_version`**: the host rejects agents with a different
  version. We provide `synapse migrate <old.wasm> <new.wasm>` (planned,
  not yet implemented) to upgrade agents in place.
  *跨 `abi_version`：宿主拒绝版本不同的 agent。我们计划提供 `synapse migrate <old.wasm> <new.wasm>`（计划中，尚未实现）来原地升级 agent。*
- **Deprecated symbols**: when an export is removed, the host
  continues to recognize and call the deprecated symbol during a
  deprecation window of one minor Synapse version, then stops.
  *废弃符号：某个导出被移除后，宿主会在一个 Synapse minor 版本的废弃窗口内继续识别并调用该废弃符号，之后停止。*
