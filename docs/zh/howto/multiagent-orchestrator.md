---
title: MultiAgent Orchestrator
---

# MultiAgent: Orchestrator · MultiAgent：Orchestrator

分解复杂任务，并行派发多个 sub-agent。

## 目标

构建一个多 agent 系统：LLM 分解任务，并在单次响应中派发多个 sub-agent（并行执行）。

## 适用场景

  - 研究类任务（并行从多个来源收集信息）
  - 多视角分析（从不同角度分析）
  - 包含独立子任务的工作流

## 代码

```rust
use std::sync::Arc;
use synapse::adk::agent::{Agent, AgentInput};
use synapse::adk::multiagent::OrchestratorAgent;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));

let researcher = Arc::new(researcher_agent);
let analyst = Arc::new(analyst_agent);
let writer = Arc::new(writer_agent);

let orchestrator = OrchestratorAgent::builder("orchestrator", model)
    .description("Decomposes tasks and dispatches specialists in parallel")
    .add_agent(researcher)
    .add_agent(analyst)
    .add_agent(writer)
    .max_iterations(12)
    .build();

let event = orchestrator.run(AgentInput {
    messages: vec![Message::user(
        "Research the impact of AI on healthcare, \
         analyze the economic implications, and write a summary report."
    )],
    enable_streaming: false,
}).await?;
```

## 工作原理

   1. LLM 看到复杂任务
   2. LLM 把任务分解为 3 个独立子任务
   3. LLM 在**一次响应**中发出 3 个 tool call（并行）
   4. 系统并行运行全部 3 个 sub-agent
   5. LLM 看到全部 3 个结果，综合出最终答案

## 与 Router 的区别

| 模式 | LLM 行为 |
|---------|--------------|
| Router | 顺序式："先调 A，再调 B" |
| Orchestrator | 并行式："一次性同时调用 A、B、C" |

## See Also
