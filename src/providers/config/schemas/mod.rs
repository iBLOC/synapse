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

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ **小米 MiMo 的两套 id 命名空间是故意的，但「默认模型」必须同值**
    /// （2026-11-15）：预设侧在 `catalog::presets()`（id `xiaomi-mimo`），
    /// schema 侧在这里（id `mimo`）⇒ ⇒ 两处默认模型各写各的 ⇒ ⇒ 一处活
    /// 一处死 ⇒ ⇒ 用户从预设建的供应商与从 schema 建的供应商跑出两个
    /// 行为 ⇒ ⇒ 这里把「两处同值」钉住。
    #[test]
    fn the_mimo_schema_and_preset_agree_on_the_default_model() {
        let schema = find_schema("mimo").expect("mimo schema");
        let field = schema.find_field("model").expect("model field");
        let schema_default = field
            .default
            .as_ref()
            .and_then(|v| v.as_str())
            .expect("schema model default");
        let preset_default = crate::providers::catalog::find_preset("xiaomi-mimo")
            .expect("xiaomi-mimo preset")
            .default_model
            .expect("preset default model");
        assert_eq!(schema_default, preset_default);
        // ⚠️ **死模型不许回来**（`mimo-v2.5-pro` / `mimo-v2.5` 已于
        // 2026.10.21 10:00(UTC+8) 正式下线）。
        assert_ne!(schema_default, "mimo-v2.5-pro");
        assert_ne!(schema_default, "mimo-v2.5");
    }
}
