use std::path::{Path, PathBuf};

use super::spec::AgentSpec;

pub struct AgentLoader {
    agents_root: PathBuf,
}

impl AgentLoader {
    pub fn new(agents_root: impl Into<PathBuf>) -> Self {
        Self {
            agents_root: agents_root.into(),
        }
    }

    pub async fn load_from_directory(&self, name: &str) -> Result<AgentSpec, String> {
        let dir = self.agents_root.join(name);
        if !dir.exists() {
            return Err(format!("Agent directory not found: {:?}", dir));
        }

        let config_path = dir.join("agent.toml");
        if !config_path.exists() {
            return Err(format!("Missing agent.toml in {:?}", dir));
        }

        let config_content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("Failed to read agent.toml: {}", e))?;

        let mut spec: AgentSpec = toml::from_str(&config_content)
            .map_err(|e| format!("Failed to parse agent.toml: {}", e))?;

        let instructions_path = dir.join("instructions.md");
        if instructions_path.exists() {
            spec.instructions = tokio::fs::read_to_string(&instructions_path)
                .await
                .map_err(|e| format!("Failed to read instructions.md: {}", e))?;
        }

        spec.tools = self.load_tools(&dir).await?;
        spec.skills = self.load_skills(&dir).await?;

        Ok(spec)
    }

    pub async fn load_all(&self) -> Result<Vec<AgentSpec>, String> {
        let mut agents = Vec::new();

        if !self.agents_root.exists() {
            return Ok(agents);
        }

        let mut entries = tokio::fs::read_dir(&self.agents_root)
            .await
            .map_err(|e| format!("Failed to read agents directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(spec) = self.load_from_directory(name).await {
                        agents.push(spec);
                    }
                }
            }
        }

        Ok(agents)
    }

    async fn load_tools(&self, dir: &Path) -> Result<Vec<super::spec::ToolSpec>, String> {
        let tools_dir = dir.join("tools");
        if !tools_dir.exists() {
            return Ok(Vec::new());
        }

        let mut tools = Vec::new();
        let mut entries = tokio::fs::read_dir(&tools_dir)
            .await
            .map_err(|e| format!("Failed to read tools dir: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                tools.push(super::spec::ToolSpec {
                    name: file_name.trim_end_matches(".rs").trim_end_matches(".ts").trim_end_matches(".py").to_string(),
                    kind: super::spec::ToolKind::Builtin,
                    config: Default::default(),
                });
            }
        }

        Ok(tools)
    }

    async fn load_skills(&self, dir: &Path) -> Result<Vec<super::spec::SkillSpec>, String> {
        let skills_dir = dir.join("skills");
        if !skills_dir.exists() {
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();
        let mut entries = tokio::fs::read_dir(&skills_dir)
            .await
            .map_err(|e| format!("Failed to read skills dir: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    skills.push(super::spec::SkillSpec {
                        name: file_name.trim_end_matches(".md").to_string(),
                        path: path.clone(),
                        description: None,
                    });
                }
            }
        }

        Ok(skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = AgentLoader::new("/tmp/agents");
        assert_eq!(loader.agents_root.to_str(), Some("/tmp/agents"));
    }
}