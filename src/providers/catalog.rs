//! Provider preset catalog & model listing.
//!
//! Every provider in the platform speaks one of two API standards:
//!
//! * [`ApiStandard::OpenAi`] — OpenAI-compatible `/v1/*` surface
//!   (`Authorization: Bearer <key>`, models at `GET {base}/models`).
//! * [`ApiStandard::Claude`] — Anthropic surface
//!   (`x-api-key` + `anthropic-version`, models at `GET {base}/v1/models`).
//!
//! A [`ProviderPreset`] bundles the request URL and the API standard so a
//! consumer (shadow-desktop, tiangong-agent, …) only needs to ask the user
//! for an API key. Custom providers supply name / base URL / standard / key
//! themselves and reuse [`list_models`] unchanged.
//!
//! This module is intentionally free of UI concerns: it is the reusable
//! substrate the 端脑 (encephalon) `ProviderManager` layer builds on.

use serde::{Deserialize, Serialize};

/// Anthropic API version header required by the Claude standard.
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// The two API standards providers expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiStandard {
    /// OpenAI-compatible: Bearer auth, `GET {base}/models`.
    OpenAi,
    /// Anthropic-compatible: `x-api-key` + `anthropic-version`,
    /// `GET {base}/v1/models`.
    Claude,
}

impl ApiStandard {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiStandard::OpenAi => "openai",
            ApiStandard::Claude => "claude",
        }
    }

    /// Parse from the wire form (`"openai"` / `"claude"`), case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "openai" => Some(ApiStandard::OpenAi),
            "claude" | "anthropic" => Some(ApiStandard::Claude),
            _ => None,
        }
    }
}

impl std::fmt::Display for ApiStandard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A built-in provider preset: selecting it implies base URL + API standard,
/// so the user only supplies an API key.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    pub api_standard: ApiStandard,
    /// Suggested model used until the first `/models` fetch lands.
    pub default_model: Option<&'static str>,
    /// Local runtimes (ollama, neuron) serve models without a key.
    pub requires_key: bool,
}

static PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "xiaomi-mimo",
        name: "Xiaomi MiMo",
        base_url: "https://api.xiaomimimo.com/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("mimo-v2.5-pro"),
        requires_key: true,
    },
    ProviderPreset {
        id: "openai",
        name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("gpt-4o"),
        requires_key: true,
    },
    ProviderPreset {
        id: "anthropic",
        name: "Anthropic",
        base_url: "https://api.anthropic.com",
        api_standard: ApiStandard::Claude,
        default_model: Some("claude-sonnet-4-20250514"),
        requires_key: true,
    },
    ProviderPreset {
        id: "deepseek",
        name: "DeepSeek",
        base_url: "https://api.deepseek.com",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("deepseek-chat"),
        requires_key: true,
    },
    ProviderPreset {
        id: "minimax",
        name: "MiniMax",
        base_url: "https://api.minimax.chat/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("MiniMax-Text-01"),
        requires_key: true,
    },
    ProviderPreset {
        id: "minimax-abab",
        name: "MiniMax ABAB",
        base_url: "https://api.minimaxi.chat/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("abab6.5s-chat"),
        requires_key: true,
    },
    ProviderPreset {
        id: "qwen",
        name: "通义千问",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("qwen-plus"),
        requires_key: true,
    },
    ProviderPreset {
        id: "moonshot",
        name: "Moonshot",
        base_url: "https://api.moonshot.cn/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("moonshot-v1-8k"),
        requires_key: true,
    },
    ProviderPreset {
        id: "zhipuai",
        name: "智谱AI",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("glm-4"),
        requires_key: true,
    },
    ProviderPreset {
        id: "siliconflow",
        name: "SiliconFlow",
        base_url: "https://api.siliconflow.cn/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("deepseek-ai/DeepSeek-V3"),
        requires_key: true,
    },
    ProviderPreset {
        id: "doubao",
        name: "字节豆包",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("doubao-pro-32k"),
        requires_key: true,
    },
    ProviderPreset {
        id: "gemini",
        name: "Google Gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("gemini-2.0-flash"),
        requires_key: true,
    },
    ProviderPreset {
        id: "groq",
        name: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("llama-3.3-70b-versatile"),
        requires_key: true,
    },
    ProviderPreset {
        id: "mistral",
        name: "Mistral",
        base_url: "https://api.mistral.ai/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("mistral-large-latest"),
        requires_key: true,
    },
    ProviderPreset {
        id: "xai",
        name: "xAI Grok",
        base_url: "https://api.x.ai/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("grok-3"),
        requires_key: true,
    },
    ProviderPreset {
        id: "qianfan",
        name: "百度千帆",
        base_url: "https://aip.baidubce.com/v2",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("ernie-4.0-8k"),
        requires_key: true,
    },
    ProviderPreset {
        id: "spark",
        name: "讯飞星火",
        base_url: "https://spark-api-open.xf-yun.com/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("generalv3.5"),
        requires_key: true,
    },
    ProviderPreset {
        id: "ollama",
        name: "Ollama",
        base_url: "http://localhost:11434/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: None,
        requires_key: false,
    },
    ProviderPreset {
        id: "local",
        name: "本地模型",
        base_url: "http://localhost:8085/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: None,
        requires_key: false,
    },
];

/// The full built-in preset catalog.
pub fn presets() -> &'static [ProviderPreset] {
    PRESETS
}

/// Look up a preset by id (e.g. `"openai"`, `"anthropic"`).
pub fn find_preset(id: &str) -> Option<&'static ProviderPreset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// One entry of a provider's model list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSummary {
    pub id: String,
    /// Human-friendly name (Anthropic `display_name`, or the last path
    /// segment of namespaced OpenAI-style ids like `deepseek-ai/DeepSeek-V3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
}

/// Errors from [`list_models`].
#[derive(Debug)]
pub enum ModelListError {
    /// Transport-level failure (DNS, refused connection, timeout…).
    Http(String),
    /// The endpoint answered but rejected the request.
    Status(u16, String),
    /// The response body could not be parsed as a model list.
    Parse(String),
}

impl std::fmt::Display for ModelListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelListError::Http(msg) => write!(f, "request failed: {}", msg),
            ModelListError::Status(code, body) => {
                write!(f, "HTTP {} from model endpoint: {}", code, body)
            }
            ModelListError::Parse(msg) => write!(f, "unparseable model list: {}", msg),
        }
    }
}

impl std::error::Error for ModelListError {}

/// Build the models endpoint URL for a standard + base URL.
///
/// * OpenAI: `{base}/models` — presets already carry the `/v1` suffix.
/// * Claude: `{base}/v1/models`, tolerating bases that already end in `/v1`
///   or `/v1/` (some Anthropic-compatible gateways do).
pub fn models_endpoint(standard: ApiStandard, base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    match standard {
        ApiStandard::OpenAi => format!("{}/models", base),
        ApiStandard::Claude => {
            if base.ends_with("/v1") {
                format!("{}/models", base)
            } else {
                format!("{}/v1/models", base)
            }
        }
    }
}

/// Fetch the available models from a provider.
///
/// `api_key` may be empty/`None` for keyless local runtimes (ollama, neuron).
/// Uses a short client timeout so a dead endpoint does not stall callers.
pub async fn list_models(
    standard: ApiStandard,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<Vec<ModelSummary>, ModelListError> {
    let url = models_endpoint(standard, base_url);
    let key = api_key.map(str::trim).filter(|k| !k.is_empty());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| ModelListError::Http(e.to_string()))?;

    let mut request = client.get(&url);
    match standard {
        ApiStandard::OpenAi => {
            if let Some(key) = key {
                request = request.bearer_auth(key);
            }
        }
        ApiStandard::Claude => {
            if let Some(key) = key {
                request = request.header("x-api-key", key);
            }
            request = request.header("anthropic-version", ANTHROPIC_VERSION);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| ModelListError::Http(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ModelListError::Http(e.to_string()))?;

    if !status.is_success() {
        let snippet: String = body.chars().take(300).collect();
        return Err(ModelListError::Status(status.as_u16(), snippet));
    }

    match standard {
        ApiStandard::OpenAi => parse_openai_models(&body),
        ApiStandard::Claude => parse_claude_models(&body),
    }
}

/// Parse an OpenAI-standard `{"data": [...]}` models response.
///
/// Tolerates the field drift across OpenAI-compatible vendors: context
/// length appears as `context_window` / `context_length` / `max_context_length`.
pub fn parse_openai_models(body: &str) -> Result<Vec<ModelSummary>, ModelListError> {
    #[derive(Deserialize)]
    struct RawModel {
        id: String,
        #[serde(default)]
        display_name: Option<String>,
        #[serde(default)]
        created: Option<u64>,
        #[serde(default)]
        context_window: Option<u64>,
        #[serde(default)]
        context_length: Option<u64>,
        #[serde(default)]
        max_context_length: Option<u64>,
    }
    #[derive(Deserialize)]
    struct RawList {
        #[serde(default)]
        data: Vec<RawModel>,
    }

    let parsed: RawList =
        serde_json::from_str(body).map_err(|e| ModelListError::Parse(e.to_string()))?;
    Ok(parsed
        .data
        .into_iter()
        .map(|m| ModelSummary {
            display_name: Some(m.display_name.unwrap_or_else(|| last_segment(&m.id))),
            context_length: m
                .context_window
                .or(m.context_length)
                .or(m.max_context_length),
            id: m.id,
            created_at: m.created,
        })
        .collect())
}

/// Parse an Anthropic-standard `{"data": [...], "has_more": bool}` response.
pub fn parse_claude_models(body: &str) -> Result<Vec<ModelSummary>, ModelListError> {
    #[derive(Deserialize)]
    struct RawModel {
        id: String,
        #[serde(default)]
        display_name: Option<String>,
    }
    #[derive(Deserialize)]
    struct RawList {
        #[serde(default)]
        data: Vec<RawModel>,
    }

    let parsed: RawList =
        serde_json::from_str(body).map_err(|e| ModelListError::Parse(e.to_string()))?;
    Ok(parsed
        .data
        .into_iter()
        .map(|m| ModelSummary {
            display_name: Some(m.display_name.unwrap_or_else(|| m.id.clone())),
            id: m.id,
            context_length: None,
            created_at: None,
        })
        .collect())
}

fn last_segment(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_nonempty() {
        let mut ids: Vec<&str> = presets().iter().map(|p| p.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "duplicate preset ids in catalog");
        assert!(total >= 18);
        for p in presets() {
            assert!(!p.id.is_empty() && !p.base_url.is_empty());
            assert!(p.base_url.starts_with("http"), "{} base_url", p.id);
        }
    }

    #[test]
    fn anthropic_preset_speaks_claude_standard() {
        let p = find_preset("anthropic").expect("anthropic preset");
        assert_eq!(p.api_standard, ApiStandard::Claude);
        // Every other preset is OpenAI-standard.
        for p in presets() {
            if p.id != "anthropic" {
                assert_eq!(p.api_standard, ApiStandard::OpenAi, "{}", p.id);
            }
        }
    }

    #[test]
    fn local_runtimes_are_keyless() {
        for id in ["ollama", "local"] {
            let p = find_preset(id).unwrap_or_else(|| panic!("{} preset", id));
            assert!(!p.requires_key, "{} should not require a key", id);
        }
    }

    #[test]
    fn api_standard_roundtrip() {
        assert_eq!(ApiStandard::parse("openai"), Some(ApiStandard::OpenAi));
        assert_eq!(ApiStandard::parse("Claude"), Some(ApiStandard::Claude));
        assert_eq!(ApiStandard::parse(" anthropic "), Some(ApiStandard::Claude));
        assert_eq!(ApiStandard::parse("grpc"), None);
    }

    #[test]
    fn endpoint_building() {
        use ApiStandard::*;
        assert_eq!(
            models_endpoint(OpenAi, "https://api.openai.com/v1"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            models_endpoint(OpenAi, "http://localhost:11434/v1/"),
            "http://localhost:11434/v1/models"
        );
        assert_eq!(
            models_endpoint(Claude, "https://api.anthropic.com"),
            "https://api.anthropic.com/v1/models"
        );
        assert_eq!(
            models_endpoint(Claude, "https://gw.example.com/v1"),
            "https://gw.example.com/v1/models"
        );
    }

    #[test]
    fn parse_openai_vendor_variants() {
        let body = r#"{
            "object": "list",
            "data": [
                {"id": "gpt-4o", "object": "model", "created": 1715367049, "owned_by": "openai"},
                {"id": "deepseek-ai/DeepSeek-V3", "context_length": 65536},
                {"id": "qwen-max", "context_window": 32768},
                {"id": "glm-4", "max_context_length": 128000, "display_name": "GLM-4"}
            ]
        }"#;
        let models = parse_openai_models(body).unwrap();
        assert_eq!(models.len(), 4);
        assert_eq!(models[0].id, "gpt-4o");
        assert_eq!(models[0].created_at, Some(1715367049));
        // Namespaced id → display name is the last segment.
        assert_eq!(models[1].display_name.as_deref(), Some("DeepSeek-V3"));
        assert_eq!(models[1].context_length, Some(65536));
        assert_eq!(models[2].context_length, Some(32768));
        assert_eq!(models[3].context_length, Some(128000));
        assert_eq!(models[3].display_name.as_deref(), Some("GLM-4"));
    }

    #[test]
    fn parse_claude_models_shape() {
        let body = r#"{
            "data": [
                {"type": "model", "id": "claude-sonnet-4-20250514", "display_name": "Claude Sonnet 4", "created_at": "2025-05-14T00:00:00Z"},
                {"type": "model", "id": "claude-opus-4-20250514", "display_name": "Claude Opus 4", "created_at": "2025-05-14T00:00:00Z"}
            ],
            "has_more": false,
            "first_id": "claude-sonnet-4-20250514",
            "last_id": "claude-opus-4-20250514"
        }"#;
        let models = parse_claude_models(body).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "claude-sonnet-4-20250514");
        assert_eq!(models[0].display_name.as_deref(), Some("Claude Sonnet 4"));
    }

    #[test]
    fn parse_errors_are_typed() {
        assert!(matches!(
            parse_openai_models("not json"),
            Err(ModelListError::Parse(_))
        ));
        assert!(matches!(
            parse_claude_models("<html>404</html>"),
            Err(ModelListError::Parse(_))
        ));
    }

    // ---- live-free HTTP tests against a one-shot local server ----

    /// Spawn a thread that accepts exactly one HTTP connection, forwards the
    /// raw request bytes to `tx` (when requested), and replies with `response`.
    fn one_shot_server(
        response: &'static str,
    ) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut buf = vec![0u8; 16384];
                let n = sock.read(&mut buf).unwrap_or(0);
                let raw = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = tx.send(raw);
                let _ = sock.write_all(response.as_bytes());
                let _ = sock.flush();
            }
        });
        (format!("http://{}", addr), rx)
    }

    const OPENAI_LIST: &str = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"data\":[{\"id\":\"llama3\",\"object\":\"model\"}]}";

    #[tokio::test]
    async fn list_models_openai_standard_sends_bearer_and_parses() {
        let (base, rx) = one_shot_server(OPENAI_LIST);
        let models = list_models(ApiStandard::OpenAi, &base, Some(" sk-test \n"))
            .await
            .unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "llama3");

        let raw = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert!(raw.starts_with("GET /models HTTP/1.1"), "path: {}", raw);
        let headers = raw.to_ascii_lowercase();
        assert!(headers.contains("authorization: bearer sk-test"), "{}", raw);
    }

    #[tokio::test]
    async fn list_models_claude_standard_sends_api_key_headers() {
        let body = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"data\":[{\"type\":\"model\",\"id\":\"claude-3-5-haiku-20241022\",\"display_name\":\"Claude 3.5 Haiku\"}]}";
        let (base, rx) = one_shot_server(body);
        let models = list_models(ApiStandard::Claude, &base, Some("sk-ant-key"))
            .await
            .unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-3-5-haiku-20241022");

        let raw = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert!(raw.starts_with("GET /v1/models HTTP/1.1"), "path: {}", raw);
        let headers = raw.to_ascii_lowercase();
        assert!(headers.contains("x-api-key: sk-ant-key"));
        assert!(headers.contains("anthropic-version: 2023-06-01"));
    }

    #[tokio::test]
    async fn list_models_without_key_omits_auth_header() {
        let (base, rx) = one_shot_server(OPENAI_LIST);
        list_models(ApiStandard::OpenAi, &base, None)
            .await
            .unwrap();
        let raw = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert!(!raw.to_ascii_lowercase().contains("authorization"));
    }

    #[tokio::test]
    async fn list_models_surfaces_http_error_status() {
        let body = "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\n\r\n{\"error\":{\"message\":\"bad key\"}}";
        let (base, _rx) = one_shot_server(body);
        let err = list_models(ApiStandard::OpenAi, &base, Some("wrong"))
            .await
            .unwrap_err();
        match err {
            ModelListError::Status(401, body) => assert!(body.contains("bad key")),
            other => panic!("expected Status(401), got {:?}", other),
        }
    }
}
