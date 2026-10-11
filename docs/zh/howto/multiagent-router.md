---
title: MultiAgent Router
---

# MultiAgent: Router/Supervisor · MultiAgent：Router/Supervisor

用 LLM 把任务路由给专门的 sub-agent。

## 目标

构建一个多 agent 系统：由顶层 LLM 为每个用户查询挑选合适的专家。

## 使用场景

  - 客服（账单、退款、技术支持）
  - 研究助手（搜索、摘要、引用）
  - 代码助手（调试、重构、写文档）

## 代码

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

## 工作原理

   1. 用户提问
   2. LLM 把可用的 agent 视为 `route_to_<name>` tools
   3. LLM 挑选其中一个并调用其 tool
   4. sub-agent 运行并返回结果
   5. LLM 看到结果后决定：再调用另一个 agent 还是综合答案

## 嵌套

sub-agent 本身也可以是 `RouterSupervisor`（嵌套 routing）。

## See Also
