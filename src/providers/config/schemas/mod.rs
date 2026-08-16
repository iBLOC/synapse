pub mod openai;
pub mod anthropic;
pub mod mimo;

pub use openai::openai_schema;
pub use anthropic::anthropic_schema;
pub use mimo::mimo_schema;

use super::schema::ProviderSchema;

pub fn all_schemas() -> Vec<ProviderSchema> {
    vec![
        openai_schema(),
        anthropic_schema(),
        mimo_schema(),
    ]
}

pub fn find_schema(id: &str) -> Option<ProviderSchema> {
    match id {
        "openai" => Some(openai_schema()),
        "anthropic" => Some(anthropic_schema()),
        "mimo" => Some(mimo_schema()),
        _ => None,
    }
}
