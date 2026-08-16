use crate::providers::config::schema::*;

pub fn mimo_schema() -> ProviderSchema {
    ProviderSchema::new("mimo", "Xiaomi Mimo", "Xiaomi Mimo AI model")
        .with_icon("mimo")
        .with_field(
            FieldSchema::password("api_key", "API Key")
                .with_placeholder("sk-...")
                .with_help("Your Mimo API key from mimo.mi.com")
        )
        .with_field(
            FieldSchema::text("model", "Model")
                .with_placeholder("mimo-v2.5-pro")
                .with_default(serde_json::json!("mimo-v2.5-pro"))
        )
}
