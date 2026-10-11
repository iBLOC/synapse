---
title: Tool Namespace
---

# Tool Namespace

Scoped skill registration with qualified names, inspired by MCP tool naming.

## Overview

The namespace system allows organizing skills into isolated groups with qualified names:

- `mcp__github__search` — namespace=mcp, skill=github__search
  *`mcp__github__search` —— namespace=mcp，skill=github__search*
- `web__fetch` — namespace=web, skill=fetch
  *`web__fetch` —— namespace=web，skill=fetch*

## Usage

### Basic Setup

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

### Dispatch

```rust
// By qualified name
let result = router.dispatch("mcp__github__search", params).await?;

// List all
let all = router.list_all_skills();
// ["mcp__github__search", "web__fetch", ...]
```

### Namespace Operations

```rust
// List namespaces
let namespaces = router.list_namespaces();

// List skills in namespace
let skills = router.list_skills("mcp")?;

// Get specific skill
let skill = router.get_skill("mcp__github__search");
```

## Naming Convention

```
<namespace>__<skill_name>
     ↑              ↑
  group name    skill identifier
```

- Double underscore (`__`) separates namespace from skill
- Skill names can contain additional underscores
- Example: `mcp__github__search_pr` is valid

## Integration with MCP

```rust
// Register MCP server tools
for tool in mcp_server.list_tools().await? {
    let qualified = format!("mcp__{}__{}", server_name, tool.name);
    router.register_skill("mcp", Arc::new(McpToolAdapter::new(tool)))?;
}

// Agent can call any registered tool
let result = router.dispatch("mcp__github__search_repos", params).await?;
```
