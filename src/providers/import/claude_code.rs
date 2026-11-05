//! Claude Code 的配置形状。
//!
//! ## 证据边界（**先说清**）
//!
//! ⚠️ **本机没装 Claude Code**（2026-11-04 实测：~/.claude 下只有
//! telemetry 与 transcripts；settings.json / .credentials.json / .claude.json
//! **三个都不存在**）⇒ ⇒ 这一族只有**合成夹具**的证据 ⇒ ⇒
//! **不是**「真机验过」⇒ ⇒ 装上之后要补真机读数。
//!
//! ## 三个文件
//!
//! - ~/.claude/settings.json ⇒ env.ANTHROPIC_API_KEY / env.ANTHROPIC_BASE_URL
//!   / env.ANTHROPIC_AUTH_TOKEN
//! - ~/.claude/.credentials.json ⇒ claudeAiOauth.accessToken（**OAuth 令牌，
//!   不是 api key** ⇒ ⇒ 归成另一个 provider，不冒充 api key）
//! - ~/.claude.json ⇒ primaryApiKey

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::{
    ExternalTool, ExternalToolConfig, ImportError, ImportedProvider, WriteReport, WriteUpdate,
    first_existing, paths, read_json, write_json_atomic,
};
use crate::providers::config::ProviderConfig;

/// Claude Code 的路径（**可注入** ⇒ ⇒ 单测能指向临时目录）。
#[derive(Debug, Clone, Default)]
pub struct ClaudePaths {
    /// settings.json（env 段）。
    pub settings: Vec<PathBuf>,
    /// .credentials.json（OAuth 令牌）。
    pub credentials: Vec<PathBuf>,
    /// ~/.claude.json（老位置的 primaryApiKey）。
    pub legacy: Vec<PathBuf>,
}

/// 按本机环境探测路径。
pub fn detect_paths() -> ClaudePaths {
    let mut settings = Vec::new();
    let mut credentials = Vec::new();
    let mut legacy = Vec::new();

    if let Some(home) = paths::home() {
        let claude = home.join(".claude");
        settings.push(claude.join("settings.json"));
        credentials.push(claude.join(".credentials.json"));
        legacy.push(home.join(".claude.json"));
    }
    if let Some(app) = paths::app_data() {
        let claude = app.join("Claude");
        settings.push(claude.join("settings.json"));
        credentials.push(claude.join(".credentials.json"));
    }

    ClaudePaths {
        settings,
        credentials,
        legacy,
    }
}

/// 这台机器上装了 Claude Code 吗。
pub fn is_installed(paths: &ClaudePaths) -> bool {
    paths
        .settings
        .iter()
        .chain(paths.credentials.iter())
        .chain(paths.legacy.iter())
        .any(|p| p.is_file())
}

/// 读 Claude Code 的凭据。
pub fn read(paths: &ClaudePaths) -> Result<ExternalToolConfig, ImportError> {
    let mut files = Vec::new();
    let mut providers = Vec::new();

    // ---- ① settings.json 的 env 段 ----------------------------------
    if let Some(settings_path) = first_existing(&paths.settings) {
        files.push(settings_path.clone());
        if let Some(doc) = read_json(&settings_path)? {
            let env = doc.get("env").and_then(|e| e.as_object());
            let env_str =
                |k: &str| -> Option<&str> { env.and_then(|e| e.get(k)).and_then(|v| v.as_str()) };

            if let Some(key) = env_str("ANTHROPIC_API_KEY") {
                let mut config = ProviderConfig::new("anthropic");
                config.api_key = Some(key.to_string());
                config.base_url = env_str("ANTHROPIC_BASE_URL").map(|s| s.to_string());
                config
                    .extra
                    .insert("external_tool".into(), json!("claude-code"));
                config
                    .extra
                    .insert("external_id".into(), json!("ANTHROPIC_API_KEY"));
                providers.push(ImportedProvider {
                    source_id: "ANTHROPIC_API_KEY".to_string(),
                    config,
                    origin: format!("{}#env.ANTHROPIC_API_KEY", settings_path.display()),
                });
            }

            // ⚠️ **ANTHROPIC_AUTH_TOKEN 与 API_KEY 是两回事** ⇒ ⇒ 单独一条。
            if let Some(token) = env_str("ANTHROPIC_AUTH_TOKEN") {
                let mut config = ProviderConfig::new("anthropic");
                config.api_key = Some(token.to_string());
                config.base_url = env_str("ANTHROPIC_BASE_URL").map(|s| s.to_string());
                config
                    .extra
                    .insert("external_tool".into(), json!("claude-code"));
                config
                    .extra
                    .insert("external_id".into(), json!("ANTHROPIC_AUTH_TOKEN"));
                providers.push(ImportedProvider {
                    source_id: "ANTHROPIC_AUTH_TOKEN".to_string(),
                    config,
                    origin: format!("{}#env.ANTHROPIC_AUTH_TOKEN", settings_path.display()),
                });
            }
        }
    }

    // ---- ② .credentials.json 的 OAuth 令牌 ----------------------------
    if let Some(credentials_path) = first_existing(&paths.credentials) {
        files.push(credentials_path.clone());
        if let Some(doc) = read_json(&credentials_path)? {
            let token = doc
                .get("claudeAiOauth")
                .and_then(|o| o.get("accessToken"))
                .and_then(|t| t.as_str());
            if let Some(token) = token {
                let mut config = ProviderConfig::new("anthropic");
                config.api_key = Some(token.to_string());
                config
                    .extra
                    .insert("external_tool".into(), json!("claude-code"));
                config
                    .extra
                    .insert("external_id".into(), json!("claudeAiOauth.accessToken"));
                config
                    .extra
                    .insert("credential_kind".into(), json!("oauth"));
                providers.push(ImportedProvider {
                    source_id: "claudeAiOauth.accessToken".to_string(),
                    config,
                    origin: format!("{}#claudeAiOauth.accessToken", credentials_path.display()),
                });
            }
        }
    }

    // ---- ③ ~/.claude.json the primaryApiKey -----------------------------
    if let Some(legacy_path) = first_existing(&paths.legacy) {
        files.push(legacy_path.clone());
        if let Some(doc) = read_json(&legacy_path)?
            && let Some(key) = doc.get("primaryApiKey").and_then(|k| k.as_str())
        {
            let mut config = ProviderConfig::new("anthropic");
            config.api_key = Some(key.to_string());
            config
                .extra
                .insert("external_tool".into(), json!("claude-code"));
            config
                .extra
                .insert("external_id".into(), json!("primaryApiKey"));
            providers.push(ImportedProvider {
                source_id: "primaryApiKey".to_string(),
                config,
                origin: format!("{}#primaryApiKey", legacy_path.display()),
            });
        }
    }

    if files.is_empty() {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::ClaudeCode.id(),
        });
    }

    providers.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    Ok(ExternalToolConfig {
        tool: ExternalTool::ClaudeCode,
        files,
        providers,
    })
}

/// 写回 Claude Code 的 settings.json（**只动 env 段**）。
///
/// ⚠️ **id 就是环境变量名**（ANTHROPIC_API_KEY）⇒ ⇒ 与别的工具
/// 「id 是账户名」不同 ⇒ ⇒ 这里明确按环境变量名处理。
pub fn write(paths: &ClaudePaths, updates: &[WriteUpdate<'_>]) -> Result<WriteReport, ImportError> {
    let Some(first) = paths.settings.first().cloned() else {
        return Err(ImportError::NotInstalled {
            tool: ExternalTool::ClaudeCode.id(),
        });
    };
    let target = first_existing(&paths.settings).unwrap_or(first);

    let mut doc = read_json(&target)?.unwrap_or_else(|| Value::Object(Map::new()));
    if !doc.is_object() {
        doc = Value::Object(Map::new());
    }
    let Some(root) = doc.as_object_mut() else {
        return Err(ImportError::Invalid(
            "claude settings.json is not an object".into(),
        ));
    };
    let env = root
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));
    if !env.is_object() {
        *env = Value::Object(Map::new());
    }
    let Some(env) = env.as_object_mut() else {
        return Err(ImportError::Invalid(
            "claude settings.json env is not an object".into(),
        ));
    };

    let mut changed = Vec::new();
    for (name, value) in updates {
        let Some(value) = value else { continue };
        env.insert((*name).to_string(), Value::String((*value).to_string()));
        changed.push((*name).to_string());
    }

    let backup = write_json_atomic(&target, &doc)?;
    Ok(WriteReport {
        tool: ExternalTool::ClaudeCode,
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
    fn reads_the_env_section_and_the_oauth_token_separately() {
        let dir = tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        std::fs::write(
            &settings,
            r#"{"env":{"ANTHROPIC_API_KEY":"sk-ant-1","ANTHROPIC_BASE_URL":"https://x.example"},"permissions":{"allow":["Bash"]}}"#,
        )
        .unwrap();
        let creds = dir.path().join(".credentials.json");
        std::fs::write(&creds, r#"{"claudeAiOauth":{"accessToken":"oauth-token"}}"#).unwrap();

        let got = read(&ClaudePaths {
            settings: vec![settings],
            credentials: vec![creds],
            legacy: vec![],
        })
        .unwrap();

        assert_eq!(got.providers.len(), 2);
        let api = got
            .providers
            .iter()
            .find(|p| p.source_id == "ANTHROPIC_API_KEY")
            .expect("the api key must be imported");
        assert_eq!(api.config.api_key.as_deref(), Some("sk-ant-1"));
        assert_eq!(api.config.base_url.as_deref(), Some("https://x.example"));
        let oauth = got
            .providers
            .iter()
            .find(|p| p.source_id == "claudeAiOauth.accessToken")
            .expect("the oauth token must be imported as its own entry");
        assert_eq!(oauth.config.api_key.as_deref(), Some("oauth-token"));
        assert_eq!(
            oauth.config.extra.get("credential_kind"),
            Some(&json!("oauth")),
            "an oauth token must not pretend to be an api key"
        );
    }

    #[test]
    fn writing_touches_only_the_env_section() {
        let dir = tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        std::fs::write(
            &settings,
            r#"{"env":{"ANTHROPIC_API_KEY":"old","OTHER":"keep"},"permissions":{"allow":["Bash"]}}"#,
        )
        .unwrap();

        let report = write(
            &ClaudePaths {
                settings: vec![settings.clone()],
                credentials: vec![],
                legacy: vec![],
            },
            &[("ANTHROPIC_API_KEY", Some("new"))],
        )
        .unwrap();

        assert_eq!(report.changed, vec!["ANTHROPIC_API_KEY".to_string()]);
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(doc["env"]["ANTHROPIC_API_KEY"], json!("new"));
        assert_eq!(doc["env"]["OTHER"], json!("keep"));
        assert_eq!(
            doc["permissions"]["allow"][0],
            json!("Bash"),
            "the other sections must survive"
        );
    }

    #[test]
    fn a_missing_tool_is_not_an_io_error() {
        let dir = tempdir().unwrap();
        let err = read(&ClaudePaths {
            settings: vec![dir.path().join("settings.json")],
            credentials: vec![],
            legacy: vec![],
        })
        .unwrap_err();
        assert!(
            matches!(err, ImportError::NotInstalled { .. }),
            "got {err:?}"
        );
    }
}
