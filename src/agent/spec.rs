use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub name: String,
    pub model: ModelSpec,
    pub instructions: String,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub skills: Vec<SkillSpec>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub schedules: Vec<ScheduleSpec>,
    #[serde(default)]
    pub subagents: Vec<String>,
    #[serde(default)]
    pub wasm: Option<WasmSpec>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl AgentSpec {
    pub fn builder() -> AgentSpecBuilder {
        AgentSpecBuilder::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub provider: String,
    pub name: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub kind: ToolKind,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Builtin,
    Http,
    Mcp,
    Wasm,
    Shell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: String,
    pub path: PathBuf,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub name: String,
    pub cron: String,
    pub action: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSpec {
    pub module_path: PathBuf,
    #[serde(default)]
    pub memory_limit_mb: Option<u32>,
    #[serde(default)]
    pub fuel_limit: Option<u64>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

#[derive(Default)]
pub struct AgentSpecBuilder {
    id: Option<String>,
    name: Option<String>,
    model: Option<ModelSpec>,
    instructions: Option<String>,
    tools: Vec<ToolSpec>,
    skills: Vec<SkillSpec>,
    channels: Vec<String>,
    schedules: Vec<ScheduleSpec>,
    subagents: Vec<String>,
    wasm: Option<WasmSpec>,
    env: HashMap<String, String>,
}

impl AgentSpecBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn model(mut self, model: ModelSpec) -> Self {
        self.model = Some(model);
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn tool(mut self, tool: ToolSpec) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn skill(mut self, skill: SkillSpec) -> Self {
        self.skills.push(skill);
        self
    }

    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channels.push(channel.into());
        self
    }

    pub fn schedule(mut self, schedule: ScheduleSpec) -> Self {
        self.schedules.push(schedule);
        self
    }

    pub fn subagent(mut self, subagent: impl Into<String>) -> Self {
        self.subagents.push(subagent.into());
        self
    }

    pub fn wasm(mut self, wasm: WasmSpec) -> Self {
        self.wasm = Some(wasm);
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Result<AgentSpec, String> {
        Ok(AgentSpec {
            id: self.id.ok_or("Missing agent id")?,
            name: self.name.ok_or("Missing agent name")?,
            model: self.model.ok_or("Missing model spec")?,
            instructions: self.instructions.ok_or("Missing instructions")?,
            tools: self.tools,
            skills: self.skills,
            channels: self.channels,
            schedules: self.schedules,
            subagents: self.subagents,
            wasm: self.wasm,
            env: self.env,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_spec_builder() {
        let spec = AgentSpec::builder()
            .id("test-agent")
            .name("Test Agent")
            .model(ModelSpec {
                provider: "openai".to_string(),
                name: "gpt-4".to_string(),
                base_url: None,
                api_key_env: Some("OPENAI_API_KEY".to_string()),
                temperature: Some(0.7),
                max_tokens: Some(4096),
            })
            .instructions("You are a helpful assistant.")
            .build()
            .unwrap();

        assert_eq!(spec.id, "test-agent");
        assert_eq!(spec.name, "Test Agent");
        assert_eq!(spec.model.provider, "openai");
    }

    #[test]
    fn test_agent_spec_missing_fields() {
        let result = AgentSpec::builder().build();
        assert!(result.is_err());
    }
}