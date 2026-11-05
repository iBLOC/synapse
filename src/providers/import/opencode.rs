//! opencode 的配置形状。
//!
//! ## 实测（本机 2026-11-04）
//!
//! 凭据文件（~/.local/share/opencode/auth.json）的形状是
//! { "<provider-id>": { "type": "api", "key": "<材料>" } }
//! ⇒ ⇒ 每条一个 id、一个类型、**一把材料** ⇒ ⇒ **没有 base_url**。
//! base_url 在**另一个文件**里：~/.config/opencode/opencode.json 的
//! provider.<id>.options.baseURL。
//!
//! ⚠️ 实测本机那份 opencode.json **没有** provider 段 ⇒ ⇒ base_url 常常缺
//! ⇒ ⇒ 导入时允许 base_url = None（调用方可以拿端脑的预设目录补）。
//!
//! ⚠️ **type 不是 api 的条目必须跳过** —— oauth 条目里没有 api key，
//! 它是 refresh / access 令牌 ⇒ ⇒ 当成 api key 导入就是错的。

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::{
    ExternalTool, ExternalToolConfig, ImportError, ImportedProvider, WriteReport, WriteUpdate,
    collect_base_urls, first_existing, paths, read_json, write_json_atomic,
};
use crate::providers::config::ProviderConfig;

/// opencode 的路径（**可注入** ⇒ ⇒ 单测能指向临时目录）。
#[derive(Debug, Clone, Default)]
pub struct OpenCodePaths {
    /// 凭据文件（auth.json）。
    pub credentials: Vec<PathBuf>,
    /// 配置文件（opencode.json，base_url 在这里）。
    pub config: Vec<PathBuf>,
}

/// 按本机环境探测路径。
pub fn detect_paths() -> OpenCodePaths {
    let mut credentials = Vec::new();
    let mut config = Vec::new();

    if let Some(d) = paths::local_app_data() {
        credentials.push(d.join("opencode").join("auth.json"));
        config.push(d.join("opencode").join("opencode.json"));
    }
    if let Some(d) = paths::data_home() {
        credentials.push(d.join("opencode").join("auth.json"));
    }
    if let Some(d) = paths::config_home() {
        credentials.push(d.join("opencode").join("auth.json"));
        config.push(d.join("opencode").join("opencode.json"));
    }

    OpenCodePaths {
        credentials,
        config,
    }
}

/// 这台机器上装了 opencode 吗。
pub fn is_installed(paths: &OpenCodePaths) -> bool {
    paths.credentials.iter().any(|p| p.is_file())
}

/// 读 opencode 的供应商。
pub fn read(paths: &OpenCodePaths) -> Result<ExternalToolConfig, ImportError> {
    let Some(credential_path) = first_existing(&paths.credentials) else {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::OpenCode.id(),
        });
    };

    let doc = read_json(&credential_path)?.unwrap_or_else(|| Value::Object(Map::new()));

    let mut files = vec![credential_path.clone()];
    let mut base_urls = std::collections::BTreeMap::new();
    if let Some(config_path) = first_existing(&paths.config) {
        files.push(config_path.clone());
        if let Some(cfg) = read_json(&config_path)? {
            base_urls = collect_base_urls(&cfg, "provider");
        }
    }

    let mut providers = Vec::new();
    if let Some(map) = doc.as_object() {
        for (id, entry) in map {
            let kind = entry.get("type").and_then(|t| t.as_str()).unwrap_or("api");
            if kind != "api" {
                continue;
            }
            let Some(key) = entry.get("key").and_then(|k| k.as_str()) else {
                continue;
            };
            // ⚠️⚠️ **`provider` 是「协议族」，不是外部 id** —— 外部 id 在
            //    `source_id` 与 `extra["external_id"]` 里 ⇒ ⇒ 协议族才是
            //    「这条供应商该按哪个 API 标准说话」的答案（opencode 的供应商
            //    全是 OpenAI 兼容）⇒ ⇒ 写成外部 id ⇒ ⇒ 调用方无从判断标准
            //    ⇒ ⇒ 「导进来了却用不了」⇒ ⇒ 而失败长得像成功。
            let mut config = ProviderConfig::new("openai");
            config.api_key = Some(key.to_string());
            config.base_url = base_urls.get(id).cloned();
            config
                .extra
                .insert("external_tool".into(), json!("opencode"));
            config.extra.insert("external_id".into(), json!(id));
            providers.push(ImportedProvider {
                source_id: id.clone(),
                config,
                origin: format!("{}#{}", credential_path.display(), id),
            });
        }
    }
    providers.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    Ok(ExternalToolConfig {
        tool: ExternalTool::OpenCode,
        files,
        providers,
    })
}

/// 写回 opencode 的凭据文件。
///
/// ⚠️ **只改 key 字段** ⇒ ⇒ 其余条目、其余字段原样保留。
pub fn write(
    paths: &OpenCodePaths,
    updates: &[WriteUpdate<'_>],
) -> Result<WriteReport, ImportError> {
    let Some(first) = paths.credentials.first().cloned() else {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::OpenCode.id(),
        });
    };
    // ⚠️ **已存在的优先** ⇒ ⇒ 别把新材料写到一个新文件里、而旧文件还在。
    let target = first_existing(&paths.credentials).unwrap_or(first);

    let mut doc = read_json(&target)?.unwrap_or_else(|| Value::Object(Map::new()));
    if !doc.is_object() {
        doc = Value::Object(Map::new());
    }
    let Some(root) = doc.as_object_mut() else {
        return Err(ImportError::Invalid(
            "opencode auth.json is not an object".into(),
        ));
    };

    let mut changed = Vec::new();
    for (id, key) in updates {
        let Some(key) = key else { continue };
        match root.get_mut(*id) {
            Some(entry) => {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert("key".into(), Value::String((*key).to_string()));
                }
            }
            None => {
                root.insert((*id).to_string(), json!({ "type": "api", "key": key }));
            }
        }
        changed.push((*id).to_string());
    }

    let backup = write_json_atomic(&target, &doc)?;
    Ok(WriteReport {
        tool: ExternalTool::OpenCode,
        changed,
        backup,
        written: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn reads_api_entries_and_skips_oauth() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(
            &auth,
            r#"{"minimax-cn":{"type":"api","key":"sk-aaa"},"oauth-one":{"type":"oauth","refresh":"r","access":"a"}}"#,
        )
        .unwrap();
        let cfg = dir.path().join("opencode.json");
        std::fs::write(
            &cfg,
            r#"{"provider":{"minimax-cn":{"options":{"baseURL":"https://api.example.com"}}}}"#,
        )
        .unwrap();

        let got = read(&OpenCodePaths {
            credentials: vec![auth],
            config: vec![cfg],
        })
        .unwrap();

        assert_eq!(got.providers.len(), 1, "the oauth entry must be skipped");
        assert_eq!(got.providers[0].source_id, "minimax-cn");
        assert_eq!(
            got.providers[0].config.api_key.as_deref(),
            Some("sk-aaa"),
            "the material must survive the import"
        );
        assert_eq!(
            got.providers[0].config.base_url.as_deref(),
            Some("https://api.example.com"),
            "base_url comes from the other file"
        );
    }

    #[test]
    fn writing_keeps_the_other_entries_untouched() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(
            &auth,
            r#"{"keep-me":{"type":"api","key":"old"},"change-me":{"type":"api","key":"old"}}"#,
        )
        .unwrap();

        let report = write(
            &OpenCodePaths {
                credentials: vec![auth.clone()],
                config: vec![],
            },
            &[
                ("change-me", Some("new")),
                ("fresh", Some("brand-new")),
                ("untouched", None),
            ],
        )
        .unwrap();

        assert_eq!(report.changed.len(), 2, "None must not count as a change");
        let backup = report.backup.expect("the original must be backed up");

        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&auth).unwrap()).unwrap();
        assert_eq!(doc["keep-me"]["key"], json!("old"));
        assert_eq!(doc["change-me"]["key"], json!("new"));
        assert_eq!(doc["fresh"]["key"], json!("brand-new"));
        assert!(
            doc.get("untouched").is_none(),
            "None must not create an entry"
        );

        let old: Value = serde_json::from_str(&std::fs::read_to_string(&backup).unwrap()).unwrap();
        assert_eq!(
            old["change-me"]["key"],
            json!("old"),
            "the backup must hold the original"
        );
    }

    #[test]
    fn a_missing_tool_is_not_an_io_error() {
        let dir = tempdir().unwrap();
        let err = read(&OpenCodePaths {
            credentials: vec![dir.path().join("nope.json")],
            config: vec![],
        })
        .unwrap_err();
        assert!(
            matches!(err, ImportError::NotInstalled { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn the_debug_output_never_contains_the_material() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(&auth, r#"{"p":{"type":"api","key":"sk-SECRET-VALUE"}}"#).unwrap();

        let got = read(&OpenCodePaths {
            credentials: vec![auth],
            config: vec![],
        })
        .unwrap();
        let rendered = format!("{:?}", got.providers[0]);
        assert!(
            !rendered.contains("sk-SECRET-VALUE"),
            "the material must not reach the logs: {rendered}"
        );
    }

    /// ⚠️ **`provider` 必须是协议族** —— 外部 id 在 `source_id` 里。
    ///
    /// ⚠️ **为什么钉住**：写成外部 id（"minimax-cn"）⇒ ⇒ 调用方
    /// （桌面端 / 移动端）**判断不出该按哪个 API 标准说话** ⇒ ⇒
    /// ⇒ 导进来的东西「加进去了但用不了」⇒ ⇒ 而失败长得像成功。
    #[test]
    fn the_provider_field_is_the_protocol_not_the_external_id() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(&auth, r#"{"minimax-cn":{"type":"api","key":"sk-aaa"}}"#).unwrap();

        let got = read(&OpenCodePaths {
            credentials: vec![auth],
            config: vec![],
        })
        .unwrap();

        assert_eq!(got.providers.len(), 1);
        assert_eq!(
            got.providers[0].config.provider, "openai",
            "the provider field must be the protocol family — the external id lives in source_id"
        );
        assert_eq!(got.providers[0].source_id, "minimax-cn");
    }
}
