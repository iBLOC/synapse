use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub fields: Vec<FieldSchema>,
    pub models: Vec<ModelPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub id: String,
    pub label: String,
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Password,
    Url,
    Number,
    Select { options: Vec<SelectOption> },
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub supports_streaming: bool,
    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_vision: bool,
}

impl ProviderSchema {
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            icon: None,
            fields: Vec::new(),
            models: Vec::new(),
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_field(mut self, field: FieldSchema) -> Self {
        self.fields.push(field);
        self
    }

    pub fn with_model(mut self, model: ModelPreset) -> Self {
        self.models.push(model);
        self
    }

    pub fn find_field(&self, id: &str) -> Option<&FieldSchema> {
        self.fields.iter().find(|f| f.id == id)
    }

    pub fn sensitive_fields(&self) -> Vec<&FieldSchema> {
        self.fields.iter().filter(|f| f.sensitive || matches!(f.field_type, FieldType::Password)).collect()
    }
}

impl FieldSchema {
    pub fn text(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            field_type: FieldType::Text,
            required: false,
            default: None,
            placeholder: None,
            help_text: None,
            sensitive: false,
        }
    }

    pub fn password(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            field_type: FieldType::Password,
            required: true,
            default: None,
            placeholder: None,
            help_text: None,
            sensitive: true,
        }
    }

    pub fn url(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            field_type: FieldType::Url,
            required: false,
            default: None,
            placeholder: None,
            help_text: None,
            sensitive: false,
        }
    }

    pub fn select(id: impl Into<String>, label: impl Into<String>, options: Vec<SelectOption>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            field_type: FieldType::Select { options },
            required: false,
            default: None,
            placeholder: None,
            help_text: None,
            sensitive: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn with_help(mut self, text: impl Into<String>) -> Self {
        self.help_text = Some(text.into());
        self
    }

    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

impl ModelPreset {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            max_tokens: None,
            supports_streaming: false,
            supports_tools: false,
            supports_vision: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn with_streaming(mut self) -> Self {
        self.supports_streaming = true;
        self
    }

    pub fn with_tools(mut self) -> Self {
        self.supports_tools = true;
        self
    }

    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }
}
