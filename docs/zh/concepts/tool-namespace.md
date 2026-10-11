---
title: Tool Namespace
---

# 工具命名空间

用限定名（qualified name）实现带作用域的 skill 注册，灵感来自 MCP 的工具命名。

## 概述

命名空间系统允许用限定名把 skill 组织进相互隔离的分组：

## 用法

### 基本设置

```rust
use synapse::skills::ScopedSkillRouter;

let mut router = ScopedSkillRouter::new();

// Create namespaces
router.register_namespace("mcp")?;
router.register_namespace("web")?;

// Register skills
router.register_skill("mcp", Arc::new(github_skill))?;
router.register_skill("web", Arc::new(fetch_skill))?;
```

### 分发

```rust
// By qualified name
let result = router.dispatch("mcp__github__search", params).await?;

// List all
let all = router.list_all_skills();
// ["mcp__github__search", "web__fetch", ...]
```

### 命名空间操作

```rust
// List namespaces
let namespaces = router.list_namespaces();

// List skills in namespace
let skills = router.list_skills("mcp")?;

// Get specific skill
let skill = router.get_skill("mcp__github__search");
```

## 命名约定

```
<namespace>__<skill_name>
     ↑              ↑
  group name    skill identifier
```

  - 双下划线（`__`）分隔 namespace 与 skill
  - skill 名中还可以包含额外的下划线
  - 例如 `mcp__github__search_pr` 是合法的

## 与 MCP 集成

```rust
// Register MCP server tools
for tool in mcp_server.list_tools().await? {
    let qualified = format!("mcp__{}__{}", server_name, tool.name);
    router.register_skill("mcp", Arc::new(McpToolAdapter::new(tool)))?;
}

// Agent can call any registered tool
let result = router.dispatch("mcp__github__search_repos", params).await?;
```
