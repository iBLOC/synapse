# Building a github-search sub-agent

> Reference implementation of the **agent = LLM + skill + MCP + prompt**
> pattern, on top of Synapse. This is the canonical example for any
> single-purpose sub-agent (github-search, git-insights, doc-extract,
> ...) that lives inside a multi-agent system like `prefrontal/`.

## What this is

A github-search sub-agent is a self-contained agent that knows how to
find code in github repositories. It's used by the prefrontal
supervisor when a higher-level task decomposes into "go search github
for X" sub-tasks.

The agent is **self-aware**: it knows to use the github MCP server
for github tasks. This knowledge is baked into the prompt at build
time — the supervisor does NOT need to tell it which tool to use.

## The four ingredients

```text
agent = LLM + skill + MCP + prompt
        │    │      │      └─ system prompt: "you are a github
        │    │      │         code search specialist; use the
        │    │      │         github MCP for any github task"
        │    │      └─ MCP: github MCP server (preset)
        │    └─ skills: list_issues, read_file, search_code (optional)
        └─ LLM: cortex's /neure/v1/chat/completions
```

## File layout

```
prefrontal/
├── agents/
│   └── github_search/                ← this reference
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs                ← AgentSpec + tools + MCP wiring
│       │   └── main.rs
│       ├── wit/
│       │   └── world.wit
│       └── README.md
├── src/cluster/
│   ├── mod.rs
│   └── supervisor.rs                 ← loads the agent at startup
└── src/agent_spec.rs                 ← AgentSpec struct
```

## AgentSpec

```rust
// In prefrontal/src/agent_spec.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skills: Vec<String>,
    pub system_prompt: String,
    pub llm: LlmConfig,
    pub tools: Vec<ToolSpec>,         // built-in Rust functions
    pub mcps: Vec<McpServerConfig>,   // preset MCP servers
    pub wasm: Option<WasmPackageRef>, // optional: this agent is a WASM module
}
```

The github-search sub-agent's spec:

```rust
AgentSpec {
    id: "github-search",
    name: "GitHub Code Search Specialist",
    description: "Searches github code, lists issues, summarizes PRs",
    skills: vec!["read_file".into(), "list_issues".into()],
    system_prompt: "You are a github code search specialist. \
        When asked about code in github repositories, ALWAYS use the \
        github MCP server's `search_code`, `list_issues`, and \
        `read_file` tools. Never invent repository contents — always \
        query the MCP. Return your findings as a structured JSON \
        list.".into(),
    llm: LlmConfig::default(),         // talks to cortex's /neure/v1/
    tools: vec![/* read_file stub */],
    mcps: vec![McpServerConfig::Http {
        url: "https://github-mcp.example.com".into(),
        auth: Some("token-xyz".into()),
    }],
    wasm: Some(WasmPackageRef::new("github-search", "0.1.0")),
}
```

The system prompt is the **only place** where "use github MCP for github
tasks" lives. It's baked in. New instances of the same sub-agent
inherit this knowledge automatically.

## Loading the agent at startup

```rust
// In prefrontal/main.rs
let supervisor = Supervisor::new(runtime.clone(), registry.clone());

supervisor.register_agent(github_search_spec())?;
supervisor.register_agent(git_insights_spec())?;
supervisor.register_agent(doc_extract_spec())?;

// On task dispatch:
let task = TaskRequest {
    task_id: Uuid::new_v4().to_string(),
    agent_id: "github-search".into(),
    payload: json!({"query": "find synapse-rs/synapse", "limit": 5}),
    source: TaskSource::Local,
};
let result = supervisor.dispatch(task).await?;
```

## How the agent runs

1. Supervisor picks the right sub-agent for the task (in this case
   the prefrontal supervisor already decided "this needs github-search")
2. Supervisor hands the payload to the agent's `stream_run`
3. The agent's prompt instructs it to use github MCP — it
   **knows** which tools to call
4. Agent emits a stream of `AgentStreamEvent`s:
   - `TokenDelta` — its reasoning ("I'll search for synapse-rs/synapse…")
   - `ToolStart` — calling `search_code` MCP tool
   - `ToolEnd` — got results
   - `Final` — final answer (formatted)
5. Supervisor returns the Final to the caller

## MCP server selection

The agent uses the **first MCP server that has the right tool**:

- `search_code`, `read_file`, `list_issues` → github MCP
- `read_url`, `web_search` → fetch MCP
- `vector_search`, `query_kg` → memory lobe MCP

This is **baked into the agent's preset**, not decided at runtime.

## What's NOT in this reference

This is a *reference*, not a production sub-agent. To turn it into
one you'd need:

- A real github MCP server (`npx @modelcontextprotocol/server-github`)
  running somewhere the agent can reach
- A github personal access token in the agent's config
- Skill implementations (`read_file` would need to read local files
  or query a code search index)

But the **shape** of the agent (LLM + prompt + tools + MCPs + WASM
package) is correct and matches how every other sub-agent should be
built.

## See also

- `synapse/templates/agent/` — the wasm32-wasip2 template every
  sub-agent starts from
- `docs/wasm-agent-abi.md` — the WASM ABI every sub-agent implements
- `synapse/src/mcp/` — `HttpMcpClient`, `StdioMcpClient`, `McpToolAdapter`
- `prefrontal/src/cluster/supervisor.rs` — loads the agent at runtime
