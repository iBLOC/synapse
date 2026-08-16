use crate::providers::config::schema::*;

pub fn openai_schema() -> ProviderSchema {
    ProviderSchema::new("openai", "OpenAI", "OpenAI GPT models")
        .with_icon("openai")
        .with_field(
            FieldSchema::password("api_key", "API Key")
                .with_placeholder("sk-...")
                .with_help("Your OpenAI API key from platform.openai.com")
        )
        .with_field(
            FieldSchema::url("base_url", "Base URL")
                .with_default(serde_json::json!("https://api.openai.com/v1"))
                .with_help("Use Ollama or other compatible endpoints")
        )
        .with_field(
            FieldSchema::select("model", "Default Model", vec![
                SelectOption::new("gpt-4o", "GPT-4o"),
                SelectOption::new("gpt-4o-mini", "GPT-4o Mini"),
                SelectOption::new("gpt-4-turbo", "GPT-4 Turbo"),
                SelectOption::new("gpt-3.5-turbo", "GPT-3.5 Turbo"),
            ])
            .with_default(serde_json::json!("gpt-4o"))
        )
        .with_model(
            ModelPreset::new("gpt-4o", "GPT-4o")
                .with_description("Most capable model")
                .with_max_tokens(128000)
                .with_streaming()
                .with_tools()
                .with_vision()
        )
        .with_model(
            ModelPreset::new("gpt-4o-mini", "GPT-4o Mini")
                .with_description("Fast and affordable")
                .with_max_tokens(128000)
                .with_streaming()
                .with_tools()
        )
        .with_model(
            ModelPreset::new("gpt-4-turbo", "GPT-4 Turbo")
                .with_description("Previous generation flagship")
                .with_max_tokens(128000)
                .with_streaming()
                .with_tools()
                .with_vision()
        )
        .with_model(
            ModelPreset::new("gpt-3.5-turbo", "GPT-3.5 Turbo")
                .with_description("Fast and cheap")
                .with_max_tokens(16385)
                .with_streaming()
                .with_tools()
        )
}
