//! What to launch. Pure data, no I/O.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Specification of an external ACP-speaking agent subprocess.
///
/// Decoupled from the connection logic so callers can pass any of the
/// preset helpers ([`Self::opencode`], [`Self::claude_code`], [`Self::codex`])
/// or build a fully custom command.
#[derive(Debug, Clone)]
pub struct AcpAgentSpec {
    /// Display name used as the `agent` field in [`crate::adk::stream::AgentStreamEvent`]
    /// and in log lines.
    pub name: String,
    /// Executable path or `PATH`-lookup name (e.g. `"opencode"`, `"npx"`).
    pub command: PathBuf,
    /// Arguments passed verbatim after the command.
    pub args: Vec<String>,
    /// Environment variables to inject into the child process.
    ///
    /// We merge these *on top* of the inherited environment — callers
    /// usually want to pass things like `OPENCODE_API_KEY` without
    /// having to copy the rest of the parent's env.
    pub env: BTreeMap<String, String>,
}

impl AcpAgentSpec {
    /// Build a spec from a name and a command (no args, no env overrides).
    pub fn new(name: impl Into<String>, command: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    /// Append one command-line argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append many command-line arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set a single environment variable on the child process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// `opencode --acp` — the upstream-recommended way to drive OpenCode
    /// as an ACP agent.
    pub fn opencode() -> Self {
        Self::new("opencode", "opencode").arg("acp")
    }

    /// `claude-agent-acp` via `npx` — Anthropic's reference ACP shim for
    /// Claude Code.
    pub fn claude_code() -> Self {
        Self::new(
            "claude-code-acp",
            "npx",
        )
        .args(["-y", "@agentclientprotocol/claude-agent-acp@latest"])
    }

    /// `codex-acp` via `npx` — OpenAI Codex's reference ACP shim.
    pub fn codex() -> Self {
        Self::new(
            "codex-acp",
            "npx",
        )
        .args(["-y", "@agentclientprotocol/codex-acp@latest"])
    }

    /// Build the SDK's [`agent_client_protocol::AcpAgentConfig`] from this spec.
    pub(crate) fn to_sdk_config(&self) -> agent_client_protocol::AcpAgentConfig {
        let cfg = agent_client_protocol::AcpAgentConfig::new(&self.command);
        let cfg = cfg.args(self.args.iter().cloned());
        let cfg = if self.env.is_empty() {
            cfg
        } else {
            cfg.envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        };
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_preset_uses_acp_arg() {
        let s = AcpAgentSpec::opencode();
        assert_eq!(s.name, "opencode");
        assert_eq!(s.command, PathBuf::from("opencode"));
        assert_eq!(s.args, vec!["acp".to_string()]);
    }

    #[test]
    fn builder_chain_accumulates_args_and_env() {
        let s = AcpAgentSpec::new("custom", "node")
            .arg("agent.js")
            .args(["--quiet", "--model", "sonnet"])
            .env("OPENCODE_API_KEY", "sk-test");
        assert_eq!(s.command, PathBuf::from("node"));
        assert_eq!(
            s.args,
            vec!["agent.js", "--quiet", "--model", "sonnet"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(s.env.get("OPENCODE_API_KEY").unwrap(), "sk-test");
    }
}