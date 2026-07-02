//! 思维网络 (Thought Network)
//!
//! 每个专业 Agent 都可以内置自己的思维链，思维链由配置组合可以互相引用，
//! 实现**网状思维网络**。同时在任务探索中融入 Arbor 的假设树模式。
//!
//! # 核心概念
//!
//! - **ThoughtNode** — 思维节点 (Agent / Tool / Workflow / Condition / SubGraph)
//! - **ThoughtEdge** — 思维边 (权重 + 条件跳转)
//! - **ThoughtGraph** — 网状思维网络
//! - **ThoughtChain** — 可执行路径
//! - **WeightedScheduler** — 基于权重调度
//! - **HypothesisTree** — Arbor 风格探索
//!
//! # 思维定式
//!
//! 每个 Shadow 实例都可以有自己的思维定式 (ThoughtGraph)。
//! 当一个任务反复在某一链路成功，该链路的权重会增加，后续任务优先使用。
//! Agent 失败后，自动切换到备选链路。

pub mod types;
pub mod graph;
pub mod scheduler;
pub mod hypothesis;

pub use types::*;
pub use graph::ThoughtGraph;
pub use scheduler::{WeightedScheduler, TaskRequest, TaskAssignment, TaskResult};
pub use hypothesis::{HypothesisTree, HypothesisNode, HypothesisStatus, ArborAction};