use serde::{Deserialize, Serialize};

/// What modalities (input/output formats) this agent accepts and
/// produces. Modality strings follow the A2A v1.0 / W3C-style
/// MIME-like labels: "text", "voice" (audio), "image", "video".
pub type Modality = String;

/// Capability strings describe what the agent can DO. They are
/// free-form, lowercase, hyphen-separated. Examples:
/// - "mcp:github"        — uses github MCP
/// - "memory:recall"     — can recall from memory store
/// - "memory:store"      — can store to memory store
/// - "face-recognition"  — supports face detection/match
/// - "object-detection"  — supports object detection
/// - "sensor-ingest"     — accepts sensor stream input
/// - "voice-emotion"     — recognizes emotion in voice
/// - "image-generation"  — produces image output
pub type Capability = String;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetadata {
    /// Input modalities accepted (e.g. ["text", "voice"]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modalities: Vec<Modality>,
    /// Output modalities produced (e.g. ["text", "voice"]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modalities: Vec<Modality>,
    /// Free-form capability tags (e.g. ["mcp:github", "memory:recall"]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<Capability>,
}

impl AgentMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_inputs(mut self, mods: Vec<Modality>) -> Self {
        self.input_modalities = mods;
        self
    }

    pub fn with_outputs(mut self, mods: Vec<Modality>) -> Self {
        self.output_modalities = mods;
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<Capability>) -> Self {
        self.capabilities = caps;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    pub capabilities: Capabilities,
    /// What this agent accepts, produces, and can do. Synapse
    /// extension to the a2a-rs AgentCard; applications read this
    /// to know which modalities to use and which capabilities to
    /// expose to the user.
    #[serde(default)]
    pub metadata: AgentMetadata,
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
            metadata: AgentMetadata::default(),
            authentication: None,
            skills: None,
        }
    }

    /// 占位 AgentCard — 当 Agent 还没填充描述/URL 时使用。
    ///
    /// 替代各 Agent 中反复出现的 `AgentCard::new(name, "", "", "0.1.0")` 模式。
    /// 版本号使用 `"0.0.0"` 标识这是占位，应在 Agent 启动后填充实际元数据。
    pub fn placeholder(name: impl Into<String>) -> Self {
        Self::new(name, "", "", "0.0.0")
    }

    pub fn provider(mut self, provider: Provider) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn metadata(mut self, metadata: AgentMetadata) -> Self {
        self.metadata = metadata;
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

    /// Convert to the a2a-rs SDK's AgentCard type. Modalities are
    /// mapped to the SDK's `default_input_modes` / `default_output_modes`
    /// (synthesizing `text/plain`, `audio/wav`, etc. labels); capabilities
    /// are dropped (the SDK doesn't have a generic capabilities field).
    pub fn into_a2a_card(self) -> a2a::AgentCard {
        let supported_interfaces = if self.url.is_empty() {
            vec![]
        } else {
            // The Synapse AgentCard carries a single endpoint URL.
            // Expose it as a JSON-RPC 0.3 interface; the SDK's wire
            // is JSON-RPC 2.0 so clients that already speak JSON-RPC
            // can connect without extra negotiation.
            vec![a2a::AgentInterface {
                url: self.url.clone(),
                protocol_binding: a2a::TRANSPORT_PROTOCOL_JSONRPC.to_string(),
                protocol_version: "0.3".to_string(),
                tenant: None,
            }]
        };

        let skills = self
            .skills
            .as_ref()
            .map(|skills| {
                skills
                    .iter()
                    .map(|s| a2a::AgentSkill {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        description: String::new(),
                        tags: Vec::new(),
                        examples: None,
                        input_modes: None,
                        output_modes: None,
                        security_requirements: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let security_schemes = self
            .authentication
            .as_ref()
            .and_then(synapse_auth_to_a2a_schemes);

        a2a::AgentCard {
            name: self.name,
            description: self.description,
            version: self.version,
            supported_interfaces,
            capabilities: a2a::AgentCapabilities {
                streaming: Some(self.capabilities.streaming),
                push_notifications: Some(self.capabilities.push_notifications),
                extensions: None,
                extended_agent_card: None,
            },
            default_input_modes: modality_to_modes(&self.metadata.input_modalities),
            default_output_modes: modality_to_modes(&self.metadata.output_modalities),
            skills,
            provider: self.provider.map(|p| a2a::AgentProvider {
                organization: p.organization,
                url: p.url.unwrap_or_default(),
            }),
            documentation_url: None,
            icon_url: None,
            security_schemes,
            security_requirements: None,
            signatures: None,
        }
    }

    /// Convert from the a2a-rs SDK's AgentCard type.
    pub fn from_a2a_card(card: a2a::AgentCard) -> Self {
        // Recover the URL from the first supported interface (we
        // only ever wrote one). A remote card with multiple
        // interfaces will have its first one adopted; clients that
        // need a specific interface should rebuild from JSON.
        let url = card
            .supported_interfaces
            .first()
            .map(|i| i.url.clone())
            .unwrap_or_default();

        // Map the SDK's security schemes back into the Synapse
        // Authentication enum. Only the first recognized scheme
        // is preserved; heterogeneous scheme lists collapse to
        // the most specific one Synapse can express.
        let authentication = card
            .security_schemes
            .as_ref()
            .and_then(a2a_schemes_to_synapse_auth);

        let skills = if card.skills.is_empty() {
            None
        } else {
            Some(
                card.skills
                    .into_iter()
                    .map(|s| SkillReference {
                        id: s.id,
                        name: s.name,
                    })
                    .collect(),
            )
        };

        Self {
            name: card.name,
            description: card.description,
            url,
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
            metadata: AgentMetadata {
                input_modalities: modes_to_modality(&card.default_input_modes),
                output_modalities: modes_to_modality(&card.default_output_modes),
                capabilities: Vec::new(),
            },
            authentication,
            skills,
        }
    }
}

/// Map a Synapse `Authentication` to the a2a-rs `HashMap<String,
/// SecurityScheme>` shape. Returns `None` for `Authentication::None`
/// (no scheme published).
fn synapse_auth_to_a2a_schemes(
    auth: &Authentication,
) -> Option<std::collections::HashMap<String, a2a::SecurityScheme>> {
    use a2a::SecurityScheme;
    use std::collections::HashMap;
    let scheme = match auth {
        Authentication::None => return None,
        Authentication::ApiKey { header, value: _ } => {
            // The Synapse representation collapses header + value
            // into a single string; the SDK keeps them separate.
            // We emit a placeholder API-key scheme with the header
            // name and an empty token — clients should use mTLS or
            // out-of-band credentials for actual values.
            SecurityScheme::ApiKey(a2a::ApiKeySecurityScheme {
                location: "header".to_string(),
                name: header.clone(),
                description: Some("API key authentication".to_string()),
            })
        }
        Authentication::Bearer { token: _ } => {
            SecurityScheme::HttpAuth(a2a::HttpAuthSecurityScheme {
                scheme: "bearer".to_string(),
                description: Some("Bearer token authentication".to_string()),
                bearer_format: Some("JWT".to_string()),
            })
        }
        Authentication::Basic { username, password: _ } => {
            SecurityScheme::HttpAuth(a2a::HttpAuthSecurityScheme {
                scheme: "basic".to_string(),
                description: Some(format!("Basic auth for {username}")),
                bearer_format: None,
            })
        }
    };
    let mut map = HashMap::new();
    map.insert("default".to_string(), scheme);
    Some(map)
}

/// Inverse of [`synapse_auth_to_a2a_schemes`]: pull the first
/// recognized scheme out of an a2a-rs security map and produce
/// a Synapse `Authentication`. Unknown scheme variants collapse
/// to `Authentication::None`.
fn a2a_schemes_to_synapse_auth(
    schemes: &std::collections::HashMap<String, a2a::SecurityScheme>,
) -> Option<Authentication> {
    use a2a::SecurityScheme;
    let scheme = schemes.values().next()?;
    Some(match scheme {
        SecurityScheme::ApiKey(s) => Authentication::ApiKey {
            header: s.name.clone(),
            value: String::new(),
        },
        SecurityScheme::HttpAuth(s) if s.scheme.eq_ignore_ascii_case("bearer") => {
            Authentication::Bearer {
                token: String::new(),
            }
        }
        SecurityScheme::HttpAuth(s) if s.scheme.eq_ignore_ascii_case("basic") => {
            Authentication::Basic {
                username: String::new(),
                password: String::new(),
            }
        }
        SecurityScheme::HttpAuth(_) | SecurityScheme::OAuth2(_)
        | SecurityScheme::OpenIdConnect(_) | SecurityScheme::MutualTls(_) => {
            // Synapse doesn't have first-class variants for these.
            // Round-trip them as a bearer token (client + server
            // negotiate the actual mechanism via the documented
            // scheme name).
            Authentication::Bearer {
                token: String::new(),
            }
        }
    })
}

fn modality_to_modes(mods: &[Modality]) -> Vec<String> {
    if mods.is_empty() {
        return vec!["text/plain".to_string()];
    }
    mods.iter()
        .map(|m| match m.as_str() {
            "text" => "text/plain".to_string(),
            "voice" | "audio" => "audio/wav".to_string(),
            "image" => "image/png".to_string(),
            "video" => "video/mp4".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn modes_to_modality(modes: &[String]) -> Vec<Modality> {
    modes
        .iter()
        .map(|m| match m.as_str() {
            "text/plain" | "text" => "text".to_string(),
            s if s.starts_with("audio") => "voice".to_string(),
            s if s.starts_with("image") => "image".to_string(),
            s if s.starts_with("video") => "video".to_string(),
            other => other.to_string(),
        })
        .collect()
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

    #[test]
    fn metadata_round_trips_through_serde() {
        let meta = AgentMetadata::new()
            .with_inputs(vec!["text".into(), "voice".into()])
            .with_outputs(vec!["text".into()])
            .with_capabilities(vec!["mcp:github".into(), "memory:recall".into()]);
        let json = serde_json::to_string(&meta).unwrap();
        let back: AgentMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.input_modalities, vec!["text", "voice"]);
        assert_eq!(back.output_modalities, vec!["text"]);
        assert_eq!(back.capabilities, vec!["mcp:github", "memory:recall"]);
    }

    #[test]
    fn modality_to_modes_handles_known_modalities() {
        let modes = modality_to_modes(&[
            "text".into(),
            "voice".into(),
            "image".into(),
            "video".into(),
        ]);
        assert_eq!(
            modes,
            vec!["text/plain", "audio/wav", "image/png", "video/mp4"]
        );
    }

    #[test]
    fn modality_to_modes_defaults_to_text_plain() {
        let modes = modality_to_modes(&[]);
        assert_eq!(modes, vec!["text/plain"]);
    }

    #[test]
    fn metadata_maps_to_a2a_input_modes() {
        let card = AgentCard::new("x", "d", "u", "1.0")
            .metadata(
                AgentMetadata::new()
                    .with_inputs(vec!["text".into(), "voice".into()])
                    .with_outputs(vec!["text".into()]),
            );
        let sdk = card.into_a2a_card();
        assert_eq!(sdk.default_input_modes, vec!["text/plain", "audio/wav"]);
        assert_eq!(sdk.default_output_modes, vec!["text/plain"]);
    }
}
