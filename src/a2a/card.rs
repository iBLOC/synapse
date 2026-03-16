use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    pub capabilities: Capabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Authentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<SkillReference>>,
}

impl AgentCard {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        url: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            url: url.into(),
            version: version.into(),
            provider: None,
            capabilities: Capabilities::default(),
            authentication: None,
            skills: None,
        }
    }

    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn authentication(mut self, auth: Authentication) -> Self {
        self.authentication = Some(auth);
        self
    }

    pub fn skills(mut self, skills: Vec<SkillReference>) -> Self {
        self.skills = Some(skills);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub organization: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            streaming: false,
            push_notifications: false,
            state_transition_history: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Authentication {
    None,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { header: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReference {
    pub id: String,
    pub name: String,
}

impl SkillReference {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}
