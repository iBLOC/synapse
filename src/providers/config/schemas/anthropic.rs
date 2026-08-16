use crate::providers::config::schema::*;

pub fn anthropic_schema() -> ProviderSchema {
    ProviderSchema::new("anthropic", "Anthropic", "Anthropic Claude models")
        .with_icon("anthropic")
        .with_field(
            FieldSchema::password("api_key", "API Key")
                .with_placeholder("sk-ant-...")
                .with_help("Your Anthropic API key from console.anthropic.com")
        )
        .with_field(
            FieldSchema::url("base_url", "Base URL")
                .with_default(serde_json::json!("https://api.anthropic.com/v1"))
        )
        .with_field(
            FieldSchema::select("model", "Default Model", vec![
                SelectOption::new("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
                SelectOption::new("claude-3-5-haiku-20241022", "Claude 3.5 Haiku"),
                SelectOption::new("claude-3-opus-20240229", "Claude 3 Opus"),
            ])
            .with_default(serde_json::json!("claude-3-5-sonnet-20241022"))
        )
        .with_model(
            ModelPreset::new("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet")
                .with_description("Best balance of intelligence and speed")
                .with_max_tokens(200000)
                .with_streaming()
                .with_tools()
                .with_vision()
        )
        .with_model(
            ModelPreset::new("claude-3-5-haiku-20241022", "Claude 3.5 Haiku")
                .with_description("Fastest model")
                .with_max_tokens(200000)
                .with_streaming()
                .with_tools()
        )
        .with_model(
            ModelPreset::new("claude-3-opus-20240229", "Claude 3 Opus")
                .with_description("Most capable model")
                .with_max_tokens(200000)
                .with_streaming()
                .with_tools()
                .with_vision()
        )
}
