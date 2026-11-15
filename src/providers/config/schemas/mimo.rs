use crate::providers::config::schema::*;

pub fn mimo_schema() -> ProviderSchema {
    // ⚠️ **这个 id 是「runtime provider type」，不是预设 id**（2026-11-15）：
    // 预设 id 是 `xiaomi-mimo`（provider store 的条目 id —— 桌面/移动端
    // 用它建条目）⇒ ⇒ 而这里是 `mimo`（synapse schema ＋ 端脑
    // `llm_configure` 的 runtime type 命名空间，与 `MIMO_API_KEY` 同名族）
    // ⇒ ⇒ **两套命名空间故意不同** ⇒ ⇒ 跨命名空间改 id 会打断端脑那一侧。
    ProviderSchema::new("mimo", "Xiaomi Mimo", "Xiaomi Mimo AI model")
        .with_icon("mimo")
        .with_field(
            FieldSchema::password("api_key", "API Key")
                .with_placeholder("sk-...")
                .with_help("Your Mimo API key from mimo.mi.com")
        )
        .with_field(
            FieldSchema::text("model", "Model")
                // ⚠️ **活模型**（2026-11-15）：`mimo-v2.5-pro` 已于 2026.10.21
                // 10:00(UTC+8) 正式下线 ⇒ ⇒ 预设与 schema 两处的默认模型
                // 必须同值（一条判据钉住，见 `schemas::tests`）。
                .with_placeholder("mimo-v2.6-pro")
                .with_default(serde_json::json!("mimo-v2.6-pro"))
        )
}
