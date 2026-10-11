---
title: MultiAgent Router
---

# MultiAgent: Router/Supervisor · MultiAgent：Router/Supervisor

Route tasks to specialized sub-agents using an LLM.
*用 LLM 把任务路由给专门的 sub-agent。*

## Goal · 目标

Build a multi-agent system where a top-level LLM picks the right
specialist for each user query.
*构建一个多 agent 系统：由顶层 LLM 为每个用户查询挑选合适的专家。*

## Use Case · 使用场景

- Customer service (billing, refund, tech support)
  *客服（账单、退款、技术支持）*
- Research assistant (search, summarization, citation)
  *研究助手（搜索、摘要、引用）*
- Code helper (debug, refactor, document)
  *代码助手（调试、重构、写文档）*

## Code · 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::RouterSupervisor;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

let researcher: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("researcher", "Researches topics", model.clone())
        .with_tools(vec![Arc::new(WebSearch)])
        .build()
);

let writer: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("writer", "Writes content", model.clone())
        .build()
);

let critic: Arc<dyn Agent> = Arc::new(
    ChatModelAgent::new("critic", "Reviews content", model.clone())
        .build()
);

let supervisor = RouterSupervisor::builder("supervisor", model)
    .description("Coordinates research, writing, and review")
    .add_agent(researcher)
    .add_agent(writer)
    .add_agent(critic)
    .max_iterations(8)
    .build();

let event = supervisor.run(AgentInput {
    messages: vec![Message::user("Write a blog post about Rust async")],
    enable_streaming: false,
}).await?;
```

## How It Works · 工作原理

1. User asks question
   *用户提问*
2. LLM sees available agents as `route_to_<name>` tools
   *LLM 把可用的 agent 视为 `route_to_<name>` tools*
3. LLM picks one, calls its tool
   *LLM 挑选其中一个并调用其 tool*
4. Sub-agent runs, returns result
   *sub-agent 运行并返回结果*
5. LLM sees result, decides: call another agent or synthesize
   *LLM 看到结果后决定：再调用另一个 agent 还是综合答案*

## Nesting · 嵌套

Sub-agents can themselves be `RouterSupervisor` (nested routing).
*sub-agent 本身也可以是 `RouterSupervisor`（嵌套 routing）。*

## See Also

- [MultiAgent concepts](/concepts/multiagent)
- [Orchestrator how-to](/howto/multiagent-orchestrator) — Parallel dispatch
- [Handoff how-to](/howto/multiagent-handoff) — Conversational transfer
