# Synapse Document Type Unification — Refactor Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-28

## 背景

### 问题

Synapse 当前有两个 `Document` 类型：

```rust
// src/components/document/mod.rs
pub struct Document {
    pub id: String,
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

// src/components/retriever/mod.rs
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    // 没有 mime_type!
}
```

**问题**：
1. 用户在 `retriever::Document` 和 `document::Document` 之间困惑
2. parse 后的 mime_type 经常丢失
3. transformer 不知道 chunk 来源的 mime_type
4. 两个相同的 `Document` 名字在不同模块造成 import 错误

### 目标

合并两个类型为唯一的 `Document`，并迁移所有现有代码。

## 设计

### 决策

1. **唯一类型**：`document::Document`（已有 mime_type 字段）
2. **`retriever::Document` 删除**：改为 `pub use document::Document;` re-export
3. **新增 `Document::text()` 便捷构造函数**：默认 `mime_type = "text/plain"`
4. **现有 `Document::new()` 保持 3-arg 版本**（id, content, mime_type）

### 影响范围

需要更新的文件：

**Source (4 files)**:
- `src/components/retriever/mod.rs` — 改 `Document` 为 re-export
- `src/components/indexer/mod.rs` — 改 import
- `src/components/rag/in_memory.rs` — 改 import + index_from_text 默认 mime_type
- 3 transformer files — 改 import

**Tests (5+ files)**:
- `tests/character_splitter_test.rs` — `make_doc` helper + 1 struct literal
- `tests/recursive_character_splitter_test.rs` — `make_doc` helper
- `tests/markdown_header_splitter_test.rs` — `make_doc` helper + 1 struct literal
- `tests/rag_in_memory_test.rs` — ~7 Document literals
- (router/orchestrator/handoff/agent_tool tests 不直接用 Document)

### 风险

| 风险 | 缓解 |
|------|------|
| 现有外部用户代码（不在仓库内）使用 `retriever::Document` | 保留 re-export，外部代码 0 改动 |
| 测试忘记加 mime_type | 用 `Document::text()` helper 避免 boilerplate |
| 字段顺序变化 | Rust struct literal 用 field name，顺序无关 |

### 实施分解

| Step | 内容 | 估时 |
|------|------|------|
| 1 | 改 `retriever::Document` 为 re-export | 5min |
| 2 | 更新 source 4 文件的 import | 5min |
| 3 | 更新 transformer 3 文件 | 5min |
| 4 | 更新测试 helper + struct literals | 15min |
| 5 | 跑全套测试 + 修编译错误 | 15min |
| 6 | commit + merge | 10min |
| **总计** | | **~1h** |

## 不在本 spec 范围

- Document 序列化格式变化（保持向后兼容）
- Document 验证（如检查 mime_type 有效性）
- 新增 mime_type filter（在 retriever 端）

## 成功标准

- 全部测试通过（无新增，无回归）
- `crate::components::retriever::Document` 仍可用（re-export）
- `Document::text(id, content)` 便捷方法存在
