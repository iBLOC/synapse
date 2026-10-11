---
title: MultiAgent Orchestrator
---

# MultiAgent: Orchestrator · MultiAgent：Orchestrator

Decompose complex tasks and dispatch sub-agents in parallel.
*分解复杂任务，并行派发多个 sub-agent。*

## Goal · 目标

Build a multi-agent system where the LLM decomposes a task and
dispatches multiple sub-agents in a single response (parallel
execution).
*构建一个多 agent 系统：LLM 分解任务，并在单次响应中派发多个 sub-agent（并行执行）。*

## When to Use · 适用场景

- Research tasks (gather from multiple sources in parallel)
  *研究类任务（并行从多个来源收集信息）*
- Multi-perspective analysis (analyze from different angles)
  *多视角分析（从不同角度分析）*
- Workflows with independent sub-tasks
  *包含独立子任务的工作流*

## Code · 代码

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

## How It Works · 工作原理

1. LLM sees complex task
   *LLM 看到复杂任务*
2. LLM decomposes into 3 independent sub-tasks
   *LLM 把任务分解为 3 个独立子任务*
3. LLM emits 3 tool calls in **one response** (parallel)
   *LLM 在**一次响应**中发出 3 个 tool call（并行）*
4. System runs all 3 sub-agents in parallel
   *系统并行运行全部 3 个 sub-agent*
5. LLM sees all 3 results, synthesizes final answer
   *LLM 看到全部 3 个结果，综合出最终答案*

## Difference from Router · 与 Router 的区别

| Pattern / 模式 | LLM Behavior / LLM 行为 |
|---------|--------------|
| Router | Sequential, "call A, then call B" / 顺序式："先调 A，再调 B" |
| Orchestrator | Parallel, "call A, B, C all at once" / 并行式："一次性同时调用 A、B、C" |

## See Also

- [Router how-to](/howto/multiagent-router) — Sequential routing
- [MultiAgent concepts](/concepts/multiagent)
