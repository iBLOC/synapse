//! Codex（OpenAI CLI）的配置形状。
//!
//! ## 证据边界（**先说清**）
//!
//! ⚠️ **本机没装 Codex**（2026-11-04 实测：~/.codex **不存在**）⇒ ⇒
//! 这一族只有**合成夹具**的证据 ⇒ ⇒ **不是**「真机验过」⇒ ⇒
//! 装上之后要补真机读数。
//!
//! ## 两个文件
//!
//! - ~/.codex/auth.json ⇒ **OPENAI_API_KEY**（还有一段 tokens 段是 OAuth
//!   ⇒ ⇒ 跳过，不冒充 api key）
//! - ~/.codex/config.toml ⇒ model_provider = "<名字>" ＋
//!   [model_providers.<名字>] base_url = "<地址>"

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::{
    ExternalTool, ExternalToolConfig, ImportError, ImportedProvider, WriteReport, WriteUpdate,
    first_existing, paths, read_json, write_json_atomic,
};
use crate::providers::config::ProviderConfig;

/// Codex 的路径（**可注入** ⇒ ⇒ 单测能指向临时目录）。
#[derive(Debug, Clone, Default)]
pub struct CodexPaths {
    /// auth.json（api key 在这里）。
    pub auth: Vec<PathBuf>,
    /// config.toml（base_url 在这里）。
    pub config: Vec<PathBuf>,
}

/// 按本机环境探测路径。
pub fn detect_paths() -> CodexPaths {
    let mut auth = Vec::new();
    let mut config = Vec::new();

    if let Some(home) = paths::home() {
        let codex = home.join(".codex");
        auth.push(codex.join("auth.json"));
        config.push(codex.join("config.toml"));
    }
    if let Some(app) = paths::app_data() {
        let codex = app.join("Codex");
        auth.push(codex.join("auth.json"));
        config.push(codex.join("config.toml"));
    }

    CodexPaths { auth, config }
}

/// 这台机器上装了 Codex 吗。
pub fn is_installed(paths: &CodexPaths) -> bool {
    paths.auth.iter().any(|p| p.is_file())
}

/// 读 Codex 的凭据。
pub fn read(paths: &CodexPaths) -> Result<ExternalToolConfig, ImportError> {
    let Some(auth_path) = first_existing(&paths.auth) else {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::Codex.id(),
        });
    };

    let doc = read_json(&auth_path)?.unwrap_or_else(|| Value::Object(Map::new()));
    let mut files = vec![auth_path.clone()];

    // base_url 在 config.toml 里 ⇒ ⇒ 要先知道选了哪个 model_provider。
    let mut base_url: Option<String> = None;
    if let Some(config_path) = first_existing(&paths.config) {
        files.push(config_path.clone());
        base_url = read_base_url(&config_path)?;
    }

    let mut providers = Vec::new();
    if let Some(key) = doc.get("OPENAI_API_KEY").and_then(|k| k.as_str()) {
        let mut config = ProviderConfig::new("openai");
        config.api_key = Some(key.to_string());
        config.base_url = base_url;
        config.extra.insert("external_tool".into(), json!("codex"));
        config
            .extra
            .insert("external_id".into(), json!("OPENAI_API_KEY"));
        providers.push(ImportedProvider {
            source_id: "OPENAI_API_KEY".to_string(),
            config,
            origin: format!("{}#OPENAI_API_KEY", auth_path.display()),
        });
    }

    // ⚠️ **tokens 段是 OAuth** ⇒ ⇒ 不导入成 api key。

    providers.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    Ok(ExternalToolConfig {
        tool: ExternalTool::Codex,
        files,
        providers,
    })
}

/// 从 config.toml 里取「当前 model_provider 的 base_url」。
fn read_base_url(path: &PathBuf) -> Result<Option<String>, ImportError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ImportError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let table: toml::Table = toml::from_str(&raw).map_err(|e| ImportError::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    let selected = table.get("model_provider").and_then(|v| v.as_str());
    // ⚠️ **整个链放进一个闭包** —— 否则 name（第一个闭包的参数）
    // 在后面几个闭包里**已经出了作用域**（E0425）。
    let url = selected.and_then(|name| {
        table
            .get("model_providers")
            .and_then(|v| v.as_table())
            .and_then(|m| m.get(name))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("base_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });
    Ok(url)
}

/// 写回 Codex 的 auth.json（**只动 OPENAI_API_KEY**）。
pub fn write(paths: &CodexPaths, updates: &[WriteUpdate<'_>]) -> Result<WriteReport, ImportError> {
    let Some(first) = paths.auth.first().cloned() else {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::Codex.id(),
        });
    };
    let target = first_existing(&paths.auth).unwrap_or(first);

    let mut doc = read_json(&target)?.unwrap_or_else(|| Value::Object(Map::new()));
    if !doc.is_object() {
        doc = Value::Object(Map::new());
    }
    let Some(root) = doc.as_object_mut() else {
        return Err(ImportError::Invalid(
            "codex auth.json is not an object".into(),
        ));
    };

    let mut changed = Vec::new();
    for (name, value) in updates {
        let Some(value) = value else { continue };
        root.insert((*name).to_string(), Value::String((*value).to_string()));
        changed.push((*name).to_string());
    }

    let backup = write_json_atomic(&target, &doc)?;
    Ok(WriteReport {
        tool: ExternalTool::Codex,
        changed,
        backup,
        written: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TOML_SAMPLE: &str = r#"model_provider = "myproxy"

[model_providers.myproxy]
base_url = "https://proxy.example/v1"
"#;

    #[test]
    fn reads_the_api_key_and_the_base_url_from_the_toml() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(&auth, r#"{"OPENAI_API_KEY":"sk-codex-1"}"#).unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(&config, TOML_SAMPLE).unwrap();

        let got = read(&CodexPaths {
            auth: vec![auth],
            config: vec![config],
        })
        .unwrap();

        assert_eq!(got.providers.len(), 1);
        assert_eq!(
            got.providers[0].config.api_key.as_deref(),
            Some("sk-codex-1")
        );
        assert_eq!(
            got.providers[0].config.base_url.as_deref(),
            Some("https://proxy.example/v1")
        );
    }

    #[test]
    fn the_oauth_token_block_is_not_imported_as_an_api_key() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(
            &auth,
            r#"{"tokens":{"id_token":"x","access_token":"y","refresh_token":"z"}}"#,
        )
        .unwrap();

        let got = read(&CodexPaths {
            auth: vec![auth],
            config: vec![],
        })
        .unwrap();
        assert!(
            got.providers.is_empty(),
            "an oauth token block must not become an api key"
        );
    }

    #[test]
    fn writing_touches_only_the_named_key() {
        let dir = tempdir().unwrap();
        let auth = dir.path().join("auth.json");
        std::fs::write(
            &auth,
            r#"{"OPENAI_API_KEY":"old","tokens":{"access_token":"keep"}}"#,
        )
        .unwrap();

        let report = write(
            &CodexPaths {
                auth: vec![auth.clone()],
                config: vec![],
            },
            &[("OPENAI_API_KEY", Some("new"))],
        )
        .unwrap();

        assert_eq!(report.changed, vec!["OPENAI_API_KEY".to_string()]);
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&auth).unwrap()).unwrap();
        assert_eq!(doc["OPENAI_API_KEY"], json!("new"));
        assert_eq!(doc["tokens"]["access_token"], json!("keep"));
    }

    #[test]
    fn a_missing_tool_is_not_an_io_error() {
        let dir = tempdir().unwrap();
        let err = read(&CodexPaths {
            auth: vec![dir.path().join("auth.json")],
            config: vec![],
        })
        .unwrap_err();
        assert!(
            matches!(err, ImportError::NotInstalled { .. }),
            "got {err:?}"
        );
    }
}
