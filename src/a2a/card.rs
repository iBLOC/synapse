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

    /// Convert to the a2a-rs SDK's AgentCard type.
    pub fn into_a2a_card(self) -> a2a::AgentCard {
        a2a::AgentCard {
            name: self.name,
            description: self.description,
            version: self.version,
            supported_interfaces: vec![], // TODO: map interfaces
            capabilities: a2a::AgentCapabilities {
                streaming: Some(self.capabilities.streaming),
                push_notifications: Some(self.capabilities.push_notifications),
                extensions: None,
                extended_agent_card: None,
            },
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![], // TODO: map skills
            provider: self.provider.map(|p| a2a::AgentProvider {
                organization: p.organization,
                url: p.url.unwrap_or_default(),
            }),
            documentation_url: None,
            icon_url: None,
            security_schemes: None,
            security_requirements: None,
            signatures: None,
        }
    }

    /// Convert from the a2a-rs SDK's AgentCard type.
    pub fn from_a2a_card(card: a2a::AgentCard) -> Self {
        Self {
            name: card.name,
            description: card.description,
            url: String::new(), // SDK doesn't have url field
            version: card.version,
            provider: card.provider.map(|p| Provider {
                organization: p.organization,
                url: Some(p.url),
            }),
            capabilities: Capabilities {
                streaming: card.capabilities.streaming.unwrap_or(false),
                push_notifications: card.capabilities.push_notifications.unwrap_or(false),
                state_transition_history: false, // SDK doesn't have this field
            },
            authentication: None, // TODO: map authentication
            skills: None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_card_into_a2a_preserves_required_fields() {
        let card = AgentCard::new("greeting-bot", "Says hello.", "https://x.test", "1.0.0");
        let sdk_card = card.into_a2a_card();

        assert_eq!(sdk_card.name, "greeting-bot");
        assert_eq!(sdk_card.description, "Says hello.");
        assert_eq!(sdk_card.version, "1.0.0");
        assert_eq!(sdk_card.default_input_modes, vec!["text/plain".to_string()]);
        assert_eq!(sdk_card.default_output_modes, vec!["text/plain".to_string()]);
        assert_eq!(
            sdk_card.capabilities.streaming,
            Some(false)
        );
    }

    #[test]
    fn agent_card_from_a2a_handles_missing_optional_fields() {
        let sdk_card = a2a::AgentCard {
            name: "x".to_string(),
            description: "y".to_string(),
            version: "0.1.0".to_string(),
            supported_interfaces: vec![],
            capabilities: a2a::AgentCapabilities::default(),
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![],
            provider: None,
            documentation_url: None,
            icon_url: None,
            security_schemes: None,
            security_requirements: None,
            signatures: None,
        };

        let card = AgentCard::from_a2a_card(sdk_card);
        assert_eq!(card.name, "x");
        assert_eq!(card.description, "y");
        assert_eq!(card.version, "0.1.0");
        assert_eq!(card.url, ""); // SDK has no url field
    }

    #[test]
    fn agent_card_into_a2a_preserves_provider() {
        let card = AgentCard::new("p", "d", "u", "1.0")
            .provider(Provider {
                organization: "ACME".to_string(),
                url: Some("https://acme.test".to_string()),
            });

        let sdk_card = card.into_a2a_card();
        let provider = sdk_card.provider.expect("provider preserved");
        assert_eq!(provider.organization, "ACME");
        assert_eq!(provider.url, "https://acme.test");
    }
}
