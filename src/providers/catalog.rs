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
        // ⚠️ **默认模型必须是活模型**（2026-11-15）：`mimo-v2.5-pro` 与
        // `mimo-v2.5` 已于 2026.10.21 10:00(UTC+8) 正式下线 ⇒ ⇒ 拿死模型当
        // 默认 ⇒ ⇒ 用户新建完供应商，第一次调用就是「模型不存在」。
        default_model: Some("mimo-v2.6-pro"),
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
    // ⚠️ **MiniMax 两条按「区域」分，不按「模态」分**（2026-11-15）。
    //
    // 官方当前口径（`platform.minimaxi.com/docs/guides/quickstart-preparation.md`
    // 与 `m-plan/other-tools.md`）：
    //   * 国际站 → `https://api.minimax.io/v1`
    //   * 国内站 → `https://api.minimax.cn/v1`（Anthropic 面同形：
    //     `https://api.minimax.cn/anthropic`）
    //
    // ⇒ ⇒ 老预设的 `api.minimax.chat/v1`（国际老域）与
    // `api.minimaxi.chat/v1`（国内老域）**都不是当前口径** ⇒ ⇒ 收掉。
    //
    // ⚠️ **默认模型也换了**：`MiniMax-Text-01` / `abab6.5s-chat` 都不在
    // MiniMax 当前目录里（语言模型只有 `MiniMax-M3.1-Flash` / `MiniMax-M3`；
    // M2.5/M2.7 已于 2026-11-18 下线）⇒ ⇒ 默认模型钉 `MiniMax-M3.1-Flash`。
    //
    // ⚠️ **老的 `minimax-abab` 预设退役**（模型已下线 + 域是老域）
    // ⇒ ⇒ 存量条目自带 `api_base`（预设只管「新建时的默认」）⇒ ⇒ 不受影响。
    ProviderPreset {
        id: "minimax",
        name: "MiniMax",
        base_url: "https://api.minimax.io/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("MiniMax-M3.1-Flash"),
        requires_key: true,
    },
    ProviderPreset {
        id: "minimax-cn",
        name: "MiniMax 中国",
        base_url: "https://api.minimax.cn/v1",
        api_standard: ApiStandard::OpenAi,
        default_model: Some("MiniMax-M3.1-Flash"),
        requires_key: true,
    },
    // ⚠️ **`minimax-h3` 那条收掉了**（2026-11-16）—— 目录里的 MiniMax 只剩
    // **两条**（国际 / 国内），按**区域**分（那是真的两条线：两套域、两套
    // 计费、两种密钥）。
    //
    // 当初（2026-11-12）另立 H3 的理由是「域不同」⇒ ⇒ 2026-11-15 域对齐后
    // 这条理由没了，换成「默认模型是出图模型」⇒ ⇒ **今天这条理由也没了**：
    // 一条供应商本来就能有**多个模型**（默认模型只是其中一条）⇒ ⇒ 有了
    // `preset_models`（按官方文档预设的模型清单）⇒ ⇒ `MiniMax-H3-Image`
    // （出图）/ `MiniMax-H3`（视频）/ `speech-*`（语音）本来就列在 `minimax`
    // 这条的清单里 ⇒ ⇒ 「为了出图再单开一条供应商」这件事**不再需要**。
    //
    // ⇒ ⇒ 用户在**同一条** MiniMax 供应商里就能选到出图/视频/语音模型
    // （能力分档卡按 capability 挑）⇒ ⇒ 而「云供应商里有三四个 MiniMax」
    // 这件事**消失**。
    //
    // ⚠️ 存量条目**不受影响**（预设只管「新建时的默认」⇒ ⇒ 已经在用
    // `minimax-h3` 的那台大脑，老条目自带 api_base / 默认模型 ⇒ 照旧）。
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

/// 一条「**按官方文档预设**的模型」—— 上游**没有模型列表接口**时的兜底。
///
/// ⚠️ **为什么要有这张表**（2026-11-16 实测取证）：
/// 有的上游**压根不提供**「模型列表」这个接口（Google 的 OpenAI 兼容面
/// `GET /v1beta/openai/models` ⇒ 404；百度千帆 `GET /v2/models` ⇒ 302）
/// ⇒ ⇒ 「拉模型列表」这一格对它们**永远失败** ⇒ ⇒ 界面永远显示「尚未
/// 获取」⇒ ⇒ 「配得出来、用不起来」（失败长得像成功）。
///
/// ⇒ ⇒ 这张表按**各家的官方接口文档**把模型名抄下来 ⇒ ⇒ 拉不到时回落
/// 成它 ⇒ ⇒ 用户至少能选到一个**真模型**。
///
/// ⚠️ **只写文档里真有的 id** —— 抄错的 id 会让用户撞「模型不存在」，
/// 那比「没有清单」更坏 ⇒ ⇒ 取不到官方文档就**不填**（宁可没有，不编）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetModel {
    /// 模型 id（**原样** —— 含大小写、点号、连字符）。
    pub id: &'static str,
    /// 能力（llm / asr / tts / embedding / vision / rerank / ocr /
    /// image_gen / video_gen）。
    pub capability: &'static str,
}

/// 一个预设的「按官方文档预设的模型清单」＋**出处**。
///
/// ⚠️ **为什么是 `pub`**（2026-11-16）：读面要按**厂商**挑清单（见
/// `preset_doc_list`）⇒ ⇒ 挑中之后得把「预设 id ＋ 文档出处 ＋ 清单」
/// **一起**带走 ⇒ ⇒ 只给一个清单切片的话，调用方还得再查一次 ⇒ ⇒ 两处口径。
#[derive(Debug, Clone, Copy)]
pub struct PresetDocList {
    pub preset_id: &'static str,
    /// 官方文档地址（界面要能说「这是按哪份文档预设的」）。
    pub doc: &'static str,
    pub models: &'static [PresetModel],
}

/// MiniMax 的官方模型清单（国际站 / 国内站**同一份**）。
///
/// ⚠️ 出处：`https://platform.minimaxi.com/docs/faq/models.md`（官方「模型
/// 列表」页，2026-11-16 取）⇒ ⇒ 逐字抄的**可用**模型；已下线的 M2.x /
/// music-* **不在表里**（抄进去就是「默认拿死模型当默认」的老毛病）。
static MINIMAX_MODELS: &[PresetModel] = &[
    PresetModel { id: "MiniMax-M3.1-Flash", capability: "llm" },
    PresetModel { id: "MiniMax-M3", capability: "llm" },
    PresetModel { id: "MiniMax-H3", capability: "video_gen" },
    PresetModel { id: "MiniMax-Hailuo-2.3", capability: "video_gen" },
    PresetModel { id: "MiniMax-Hailuo-2.3-Fast", capability: "video_gen" },
    PresetModel { id: "MiniMax-Hailuo-02", capability: "video_gen" },
    PresetModel { id: "MiniMax-H3-Image", capability: "image_gen" },
    PresetModel { id: "image-01", capability: "image_gen" },
    PresetModel { id: "image-01-live", capability: "image_gen" },
    PresetModel { id: "speech-2.8-hd", capability: "tts" },
    PresetModel { id: "speech-2.8-turbo", capability: "tts" },
    PresetModel { id: "speech-2.6-hd", capability: "tts" },
    PresetModel { id: "speech-2.6-turbo", capability: "tts" },
    PresetModel { id: "speech-02-hd", capability: "tts" },
    PresetModel { id: "speech-02-turbo", capability: "tts" },
];

/// 预设 ⇒ 官方文档里的模型清单（**兜底用**，不是「实时拉的」）。
///
/// ⚠️ 键是**预设 id**（不是供应商名）⇒ ⇒ 清单跟着预设走 ⇒ ⇒ 用户把
/// base URL 改成别家时，这张表**不会被误用**。
static PRESET_DOC_LISTS: &[PresetDocList] = &[
    PresetDocList {
        preset_id: "minimax",
        doc: "https://platform.minimaxi.com/docs/faq/models.md",
        models: MINIMAX_MODELS,
    },
    PresetDocList {
        preset_id: "minimax-cn",
        doc: "https://platform.minimaxi.com/docs/faq/models.md",
        models: MINIMAX_MODELS,
    },
    PresetDocList {
        preset_id: "gemini",
        doc: "https://ai.google.dev/gemini-api/docs/models/gemini",
        models: &[
            PresetModel { id: "gemini-2.5-pro", capability: "llm" },
            PresetModel { id: "gemini-2.5-flash", capability: "llm" },
            PresetModel { id: "gemini-2.5-flash-lite", capability: "llm" },
            PresetModel { id: "gemini-3-flash-preview", capability: "llm" },
            PresetModel { id: "gemini-3.8-flash", capability: "llm" },
            PresetModel { id: "gemini-3.6-flash", capability: "llm" },
            PresetModel { id: "gemini-3.5-flash-lite", capability: "llm" },
            PresetModel { id: "gemini-3.1-flash-lite", capability: "llm" },
            PresetModel { id: "gemini-4-argon", capability: "llm" },
            PresetModel { id: "gemini-4-carbon", capability: "llm" },
            PresetModel { id: "gemini-4-hydrogen", capability: "llm" },
            PresetModel { id: "gemini-omni-flash", capability: "llm" },
            PresetModel { id: "gemini-2.5-flash-image", capability: "image_gen" },
            PresetModel { id: "gemini-3-pro-image", capability: "image_gen" },
            PresetModel { id: "gemini-3.1-flash-image", capability: "image_gen" },
            PresetModel { id: "gemini-3.5-transcribe", capability: "asr" },
            PresetModel { id: "gemini-3.8-flash-tts", capability: "tts" },
            PresetModel { id: "gemini-3.8-flash-lite-tts", capability: "tts" },
        ],
    },
    PresetDocList {
        preset_id: "qianfan",
        doc: "https://ai.baidu.com/ai-doc/WENXINWORKSHOP/4mchtzl8s",
        models: &[
            PresetModel { id: "ernie-4.5-turbo-128k-preview", capability: "llm" },
            PresetModel { id: "ernie-4.5-turbo-vl-preview", capability: "llm" },
            PresetModel { id: "ernie-4.5-vl-28b-a3b", capability: "llm" },
            PresetModel { id: "ernie-4.5-21b-a3b", capability: "llm" },
            PresetModel { id: "ernie-4.5-0.3b", capability: "llm" },
        ],
    },
];

/// 某个预设的「按官方文档预设的模型清单」—— 没有 ⇒ `None`。
///
/// ⚠️ **这不是「实时拉的可用模型」** —— 它是**按官方文档抄的目录**，
/// 用在「上游没有列表接口 / 拉列表失败」的那一格 ⇒ ⇒ 调用方必须把
/// 来源标出来（不许让人以为这是实时结果）。
pub fn preset_models(preset_id: &str) -> Option<&'static [PresetModel]> {
    PRESET_DOC_LISTS
        .iter()
        .find(|d| d.preset_id == preset_id)
        .map(|d| d.models)
}

/// 清单的**官方文档出处** —— 界面要能说「这是按哪份文档预设的」。
pub fn preset_models_doc(preset_id: &str) -> Option<&'static str> {
    PRESET_DOC_LISTS
        .iter()
        .find(|d| d.preset_id == preset_id)
        .map(|d| d.doc)
}

/// **官方域** ⇒ 预设 id —— **只登记官方域**。
///
/// ⚠️ **为什么要有这张表**（2026-11-16）：这份清单说的是**这家上游**有哪些
/// 模型 ⇒ ⇒ 归属得由**主机名**说了算（不是由「用户挑了哪个预设」说了算）
/// ⇒ ⇒ **自定义条目**只要指向官方域 ⇒ ⇒ 也认得 ⇒ ⇒ 否则「自己手填
/// `api.minimax.cn` 的那条」永远拿不到出图／视频／语音那几档。
///
/// ⚠️ **老域也登记**（`api.minimax.chat` / `api.minimaxi.com`）：它们**不是
/// 当前口径**（新建不再用）⇒ ⇒ 但**存量条目**还指着它 ⇒ ⇒ 它仍然是**同一家
/// 厂商** ⇒ ⇒ 清单照样认（不认 ⇒ 存量条目的兜底就没了）。
static OFFICIAL_HOSTS: &[(&str, &str)] = &[
    ("api.minimax.io", "minimax"),
    ("api.minimax.chat", "minimax"),
    ("api.minimax.cn", "minimax-cn"),
    ("api.minimaxi.com", "minimax-cn"),
    ("generativelanguage.googleapis.com", "gemini"),
    ("aip.baidubce.com", "qianfan"),
];

/// base URL 的**主机名**（小写、去端口）—— 认厂商用。
///
/// ⚠️ **只认主机名**（不看路径）⇒ ⇒ `https://api.minimax.cn/v1` 与
/// `https://api.minimax.cn` 是**同一家** ⇒ ⇒ 少写一个 `/v1` 不会失配。
pub fn base_host(base_url: &str) -> Option<String> {
    let rest = base_url.split_once("://").map(|(_, r)| r).unwrap_or(base_url);
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host);
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// 这条供应商该配哪份「按官方文档预设的清单」—— 认厂商，**两层**。
///
/// ⚠️ **① 主机名优先**：这份清单说的是**这家上游** ⇒ ⇒ 归属由主机名说了算
/// ⇒ ⇒ 「用户挑了 MiniMax 预设、却把地址改成别家」也不会被抄错清单 ⇒ ⇒
/// 也不会把 A 家的模型抄给 B 家。
/// ⚠️ **② 预设 id 兜底**：主机名认不出（自建代理／镜像站／老域）⇒ ⇒ 用户挑
/// 的那个预设说了算。
pub fn preset_doc_list(
    preset_id: Option<&str>,
    base_url: Option<&str>,
) -> Option<&'static PresetDocList> {
    if let Some(host) = base_url.and_then(base_host) {
        if let Some((_, pid)) = OFFICIAL_HOSTS.iter().find(|(h, _)| *h == host) {
            if let Some(list) = PRESET_DOC_LISTS.iter().find(|d| d.preset_id == *pid) {
                return Some(list);
            }
        }
    }
    let pid = preset_id?;
    PRESET_DOC_LISTS.iter().find(|d| d.preset_id == pid)
}

/// 清单里**上游没列**的那些条目 —— 读面拿它把上游那份**补齐**。
///
/// ⚠️ **为什么要补**（2026-11-16）：有的上游的「模型列表」接口**只列对话
/// 模型** —— MiniMax 的 `GET /v1/models` 逐字只给 `MiniMax-M3.1-Flash` /
/// `MiniMax-M3` ⇒ ⇒ 出图 `MiniMax-H3-Image` / 视频 `MiniMax-H3` / 语音
/// `speech-*` **压根不在那个列表里**（它们走各自端点）⇒ ⇒ 刷新一次 ⇒ ⇒
/// 清单被**整份替换** ⇒ ⇒ 那几档**消失** ⇒ ⇒ 「按能力分档」里那些档**空着**。
///
/// ⚠️ **按 id 比**（不是按顺序、不是按能力）⇒ ⇒ 上游已经列了的那条**不重复**
/// ⇒ ⇒ 上游将来把出图模型也列进那个接口 ⇒ ⇒ 这里自动就不补了。
pub fn preset_models_missing(
    list: &PresetDocList,
    upstream_ids: &[String],
) -> Vec<&'static PresetModel> {
    list.models
        .iter()
        .filter(|m| !upstream_ids.iter().any(|id| id == m.id))
        .collect()
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
    /// ⚠️ **能力**（llm / asr / tts / embedding / vision / rerank / ocr / image_gen / video_gen；world_gen 预留 —— 云端没有公认的名字形状，暂不推断）。
    ///
    /// ⚠️ 为什么要有它（2026-11-03）：内置条目（neuron 目录）里的模型**横跨七种
    /// ⇒ ⇒ 能力** ⇒ ⇒ 不带它，界面分不出「这条是聊天模型还是语音模型」。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    /// ⚠️ **盘上真的有吗** —— 内置条目里「目录里有、但**还没下载**」的模型也列
    /// ⇒ ⇒ 出来 ⇒ ⇒ 不带这个标记 ⇒ ⇒ 界面会把「选了也跑不起来」的模型当成
    /// ⇒ ⇒ 能用的 ⇒ ⇒ 「失败长得像成功」。
    ///
    /// ⚠️ 没有这个字段 ＝ **不知道** ⇒ ⇒ 客户端按「可用」处理（云供应商的模型
    /// ⇒ ⇒ 都是拉下来的 ⇒ ⇒ 那是安全的那一边）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    /// ⚠️ **这条 `capability` 从哪来**（`user` / `upstream` / `catalog` /
    /// `inferred` / `default`）—— 2026-11-13 加。
    ///
    /// ⚠️ 为什么必须带：只给最终词、不说来源 ⇒ ⇒ 「上游真给的」和「我们
    /// 猜的」在界面上长得一样 ⇒ ⇒ 猜错了没人发现（本仓「失败长得像成功」
    /// 纪律在**分类**上的落点）。用户据此判断哪条需要自己在模型管理页改。
    ///
    /// ⚠️ **只有端脑的读面会填**（它算五档判据）⇒ ⇒ 目录/解析路径留 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_source: Option<String>,
    /// ⚠️ **输入模态**（`text` / `image` / `audio` / `video` / `pdf`）——
    /// 2026-11-16 加。
    ///
    /// ⚠️ 为什么必须带：只给能力档（`llm`）⇒ ⇒ 用户**看不出这条模型吃什么**
    /// ⇒ ⇒ 「能不能传图」只能猜 ⇒ ⇒ 猜错就是「界面说一套、实际报 400」。
    ///
    /// ⚠️ **空 ＝ 不知道**（**不是**「只能吃文本」）⇒ ⇒ 界面不乱画。
    #[serde(default)]
    pub input_modalities: Vec<String>,
    /// 这份模态**从哪来**（`upstream` / `catalog` / `default`）—— 2026-11-16 加。
    ///
    /// ⚠️ 只给最终模态、不说来源 ⇒ ⇒ 「上游真给的」与「我们按官方文档预设的」
    /// 在界面上长得一样 ⇒ ⇒ 猜错了没人发现。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_modalities_source: Option<String>,
    /// ⚠️ **这条条目从哪来**（`upstream` / `preset`）—— 2026-11-16 加。
    ///
    /// ⚠️ 为什么必须带：上游的「模型列表」**只列对话模型**（MiniMax 的
    /// `GET /v1/models` 逐字只给两条语言模型 ⇒ ⇒ 出图／视频／语音模型是
    /// **读面按官方文档补进来的**）⇒ ⇒ 不标来源 ⇒ ⇒ 界面把它们说成「实时
    /// 拉的」⇒ ⇒ **来源标签说谎**。
    ///
    /// ⚠️ **只有读面会填**（合并那一步才分得出）⇒ ⇒ 解析路径留 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_source: Option<String>,
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

/// 一条上游模型（**两个标准共用**）—— 字段全可选 ⇒ ⇒ 上游少给哪个
/// 字段都**不炸**（各家的形状漂移很多）。
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
    /// ⚠ **上游真给的输入模态**（2026-11-16）—— 三种形状都认。
    ///
    /// ⚠ **以前目录层整块没读** ⇒ ⇒ `architecture.input_modalities`
    /// （OpenRouter 的真形状）在解析层就被**丢了** ⇒ ⇒ 上游真值到不了
    /// 界面 ⇒ ⇒ 来源标签会假装成「按文档预设的」（**来源标签说谎**）。
    #[serde(default)]
    architecture: Option<RawArchitecture>,
    #[serde(default)]
    input_modalities: Option<Vec<String>>,
    #[serde(default)]
    modalities: Option<RawModalities>,
}

/// 上游的 `architecture` 那一块（**OpenRouter 的模态就长在这儿**）。
#[derive(Deserialize)]
struct RawArchitecture {
    #[serde(default)]
    input_modalities: Option<Vec<String>>,
}

/// 上游的 `modalities` 那一块（另一种形状：`modalities.input`）。
#[derive(Deserialize)]
struct RawModalities {
    #[serde(default)]
    input: Option<Vec<String>>,
}

/// 这条模型**上游说的**输入模态（原值，三种形状都认 —— 口径与端脑写面
/// **同一套**，见 `input_modalities::pick_upstream_modalities`）。
fn upstream_modalities(m: &RawModel) -> Vec<String> {
    super::input_modalities::pick_upstream_modalities(
        &m.architecture
            .as_ref()
            .and_then(|a| a.input_modalities.clone())
            .unwrap_or_default(),
        &m.input_modalities.clone().unwrap_or_default(),
        &m.modalities
            .as_ref()
            .and_then(|x| x.input.clone())
            .unwrap_or_default(),
    )
}

/// 这条模型的**输入模态**（**上游真值优先** ⇒ ⇒ 出厂清单 ⇒ ⇒ 留空）。
///
/// ⚠ **上游真值优先**（2026-11-16）：上游说了就照抄 ⇒ ⇒ 来源标
/// `upstream`；上游没说 ⇒ ⇒ 走出厂清单（来源 `catalog`）；都没有 ⇒ ⇒
/// **留空**（**不许**填兜底 `text` —— 那是**端脑读面**三档判据的第三档，
/// 目录层不许替它做 ⇒ ⇒ 否则来源标签会说谎）。
///
/// ⚠ 归一（`file` ⇒ `pdf`）在**这里**做 ⇒ ⇒ 词表是封闭的 5 档 ⇒ ⇒
/// 客户端拿到就能直接显示（盘上那份**存原值**、由读面归一 —— 两个形状
/// 各司其职）。
fn effective_input_modalities(
    model_id: &str,
    upstream: Vec<String>,
) -> (Vec<String>, Option<String>) {
    let normalized: Vec<String> = upstream
        .iter()
        .filter_map(|w| super::input_modalities::normalize_modality(w))
        .map(|w| w.to_string())
        .collect();
    if !normalized.is_empty() {
        return (normalized, Some("upstream".to_string()));
    }
    if let Some(list) = super::input_modalities::catalog_input_modalities(model_id) {
        return (
            list.iter().map(|w| w.to_string()).collect(),
            Some("catalog".to_string()),
        );
    }
    // ⚠ 都不知道 ⇒ ⇒ **留空 ＝ 不知道**（**不是**「只能吃文本」）。
    (Vec::new(), None)
}

/// Parse an OpenAI-standard `{"data": [...]}` models response.
///
/// Tolerates the field drift across OpenAI-compatible vendors: context
/// length appears as `context_window` / `context_length` / `max_context_length`.
pub fn parse_openai_models(body: &str) -> Result<Vec<ModelSummary>, ModelListError> {
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
        .map(|m| {
            // ⚠️ 名字自己说清模态的那一族 ⇒ 保守推断；认不出 ⇒ None（不知道）。
            let capability = infer_capability(&m.id);
            // ⚠️ **输入模态**：**上游真值优先**（2026-11-16）—— 上游说了
            // 就照抄（归一后）⇒ ⇒ 来源标 `upstream`；上游没说 ⇒ ⇒ 走
            // 出厂清单（来源 `catalog`）；都没有 ⇒ ⇒ **留空**。
            let (modalities, modality_source) =
                effective_input_modalities(&m.id, upstream_modalities(&m));
            ModelSummary {
                display_name: Some(m.display_name.unwrap_or_else(|| last_segment(&m.id))),
                context_length: m
                    .context_window
                    .or(m.context_length)
                    .or(m.max_context_length),
                id: m.id,
                created_at: m.created,
                capability: capability.map(str::to_string),
                available: None,
                // 目录解析路径不判来源（端脑读面才判五档）。
                capability_source: None,
                input_modalities: modalities,
                input_modalities_source: modality_source,
                // ⚠️ 目录解析路径**不判条目来源**（读面合并时才分得出哪条是
                // 上游列的、哪条是按文档补的）⇒ ⇒ 这里留空。
                model_source: None,
            }
        })
        .collect())
}

/// Parse an Anthropic-standard `{"data": [...], "has_more": bool}` response.
pub fn parse_claude_models(body: &str) -> Result<Vec<ModelSummary>, ModelListError> {
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
        .map(|m| {
            // ⚠️ 同一条推断（Anthropic 目录里今天只有聊天模型 ⇒ 推断多半 None）。
            let capability = infer_capability(&m.id);
            // ⚠️ **输入模态**：同一条判据（**上游真值优先** ⇒ ⇒ 出厂清单
            // ⇒ ⇒ 留空）—— 认不出 ⇒ ⇒ 界面说「不知道」。
            let (modalities, modality_source) =
                effective_input_modalities(&m.id, upstream_modalities(&m));
            ModelSummary {
                display_name: Some(m.display_name.unwrap_or_else(|| m.id.clone())),
                id: m.id,
                context_length: None,
                created_at: None,
                capability: capability.map(str::to_string),
                available: None,
                // 目录解析路径不判来源（端脑读面才判五档）。
                capability_source: None,
                input_modalities: modalities,
                input_modalities_source: modality_source,
                // ⚠️ 目录解析路径**不判条目来源**（读面合并时才分得出哪条是
                // 上游列的、哪条是按文档补的）⇒ ⇒ 这里留空。
                model_source: None,
            }
        })
        .collect())
}

fn last_segment(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).to_string()
}

/// Infer a model's capability from its id, when the id itself names the modality.
///
/// ⚠️ **保守推断** —— 只认「名字自己说清了模态」的那一族；认不出 ⇒ `None`
/// （＝ 不知道 ⇒ 客户端按通用模型处理）。宁缺勿错：把聊天模型误标成
/// 生成模型 ⇒ 界面与下游按生成模型用它 ⇒ 必错。
///
/// 词汇表与端脑内置条目（neuron 目录）同一族：
/// `asr` / `tts` / `embedding` / `rerank` / `image_gen` / `video_gen`。
/// ⚠️ rerank 必须先于 embedding 判 —— `BAAI/bge-reranker-v2-m3` 两个族缀都有。
pub fn infer_capability(id: &str) -> Option<&'static str> {
    let id = id.to_ascii_lowercase();
    let has = |needle: &str| id.contains(needle);

    if has("rerank") {
        return Some("rerank");
    }
    if has("embedding") || has("-embed") || has("bge-") {
        return Some("embedding");
    }
    if has("whisper") {
        return Some("asr");
    }
    // ⚠️ **族缀 `-asr` 自己说清了模态**（2026-11-14）：`mimo-v2.5-asr` /
    // `qwen3-asr-flash` / `fun-asr` —— 以前这一族落在 `None` ⇒ 兜底成
    // `llm` ⇒ 分档卡的 asr 档永远看不见它（真机：小米 MiMo）。
    // 与 `tts` 同一宽法（裸子串）——真 id 里含 "asr" 却不是识别模型的，没有。
    if has("asr") {
        return Some("asr");
    }
    if has("tts") {
        return Some("tts");
    }
    // ⚠️ **`speech-` 族**（2026-11-15）：MiniMax 的 TTS 真 id 就是
    // `speech-2.8-hd` / `speech-02-turbo` / `speech-01-hd`
    // ⇒ ⇒ 名字里**没有** `tts` 字样 ⇒ ⇒ 以前落 `None` ⇒ ⇒ 兜底成
    // `llm` ⇒ ⇒ 分档卡的 tts 档永远看不见它。
    if has("speech-") {
        return Some("tts");
    }
    // ⚠️ **裸 `MiniMax-H3` 是视频模型**（2026-11-16）—— 它与出图模型
    // `MiniMax-H3-Image` **只差一个后缀** ⇒ ⇒ 下面那条 contains 口径
    // （`has("minimax-h3")`）会把它**误吃成出图** ⇒ ⇒ 这里**先**按
    // **精确 id**判（命名空间形态一并）⇒ ⇒ 视频模型不再被出图规则吃掉。
    if id == "minimax-h3" || id == "minimax/h3" || id.ends_with("/minimax-h3") {
        return Some("video_gen");
    }
    if has("dall-e")
        || has("flux")
        || has("stable-diffusion")
        || has("sd3")
        || has("seedream")
        || has("cogview")
        || has("kolors")
        || has("image-01")
        // ⚠️ **MiniMax H3 家族**（2026-11-12）：id 是
        // `MiniMax-H3-Image` / `MiniMax-H3-Image-Highspeed` /
        // `MiniMax-H3-Image-Context-IR` ⇒ 只认 `h3-image` / `minimax-h3`
        // 这两段，**不认宽泛的 `image`** —— 那也会命中视觉理解类模型
        // （如 `xxx-vl-image`），把它们标成出图模型会让下游真去调它。
        || has("h3-image")
        || has("minimax-h3")
    {
        return Some("image_gen");
    }
    // ⚠️ **MiniMax 的视频族**（2026-11-15）：`MiniMax-Hailuo-2.3` /
    // `MiniMax-Hailuo-02` 是**视频**模型 ⇒ ⇒ 与 `seedance` / `kling`
    // 同归 `video_gen`（2026-11-16 起端脑有第 10 档 `video_gen` ⇒ ⇒ **不再**折成 `image_gen`）。
    if has("video-01") || has("seedance") || has("kling") || has("hailuo") {
        return Some("video_gen");
    }
    None
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
    fn anthropic_preset_speaks_claude_standard() {        let p = find_preset("anthropic").expect("anthropic preset");
        assert_eq!(p.api_standard, ApiStandard::Claude);
        // Every other preset is OpenAI-standard.
        for p in presets() {
            if p.id != "anthropic" {
                assert_eq!(p.api_standard, ApiStandard::OpenAi, "{}", p.id);
            }
        }
    }

    /// ⚠️ **MiniMax 预设按区域分**（2026-11-15）—— 域与默认模型都钉住：
    /// 官方当前口径是国际 `api.minimax.io/v1`、国内 `api.minimax.cn/v1`；
    /// 默认模型钉 `MiniMax-M3.1-Flash`（老默认 `MiniMax-Text-01` 已不在
    /// MiniMax 目录里）。
    #[test]
    fn minimax_presets_use_the_current_official_bases() {
        let intl = find_preset("minimax").expect("minimax preset");
        assert_eq!(intl.base_url, "https://api.minimax.io/v1");
        assert_eq!(intl.default_model, Some("MiniMax-M3.1-Flash"));
        assert!(intl.requires_key);

        let cn = find_preset("minimax-cn").expect("minimax-cn preset");
        assert_eq!(cn.base_url, "https://api.minimax.cn/v1");
        assert_eq!(cn.default_model, Some("MiniMax-M3.1-Flash"));
        assert!(cn.requires_key);

        // ⚠️ **H3 不再另立一条**（2026-11-16）：出图/视频/语音模型进
        // `minimax` 这条的**官方文档清单** ⇒ ⇒ 目录里的 MiniMax 只有两条
        // （国际 / 国内）⇒ ⇒ 「云供应商里有三四个 MiniMax」这件事消失。
        assert!(find_preset("minimax-h3").is_none());
    }

    /// ⚠️ **退役的 id 不再是预设**（老域 + 下线模型 abab6.5s-chat）
    /// ⇒ ⇒ 存量条目自带 api_base（预设只管「新建时的默认」）⇒ ⇒ 不受影响。
    #[test]
    fn the_retired_minimax_abab_preset_is_gone() {
        assert!(find_preset("minimax-abab").is_none());
    }

    /// ⚠️ **目录里的 MiniMax 只有两条**（2026-11-16）—— 按**区域**分是
    /// 真的两条线（两套域 / 两套计费 / 两种密钥）⇒ ⇒ 保留；
    /// 而 H3 曾经另立一条（理由先「域不同」后「默认模型不同」）⇒ ⇒
    /// 两条理由都不成立了就收掉 ⇒ ⇒ 「三四个 MiniMax」这件事消失。
    #[test]
    fn the_catalog_has_exactly_two_minimax_presets() {
        let ids: Vec<&str> = presets()
            .iter()
            .filter(|p| p.id.contains("minimax"))
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec!["minimax", "minimax-cn"]);
    }

    /// ⚠️ **收掉 H3 那条 ≠ 把出图能力一起收掉**（2026-11-16）—— 出图 /
    /// 视频 / 语音模型必须**在** MiniMax 那条的**官方文档清单**里
    /// ⇒ ⇒ 否则「去重」就变成了「删功能」。
    #[test]
    fn the_minimax_doc_list_carries_the_image_video_and_voice_models() {
        let models = preset_models("minimax").expect("minimax doc list");
        let ids: Vec<&str> = models.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"MiniMax-H3-Image"), "image model missing");
        assert!(ids.contains(&"MiniMax-H3"), "video model missing");
        assert!(ids.contains(&"speech-2.8-hd"), "tts model missing");
        // ⚠️ **默认模型必须在清单里** —— 不在 ⇒ 用户新建完供应商，第一次
        // 调用就是「模型不存在」（配得出来、用不起来）。
        let def = find_preset("minimax")
            .expect("minimax preset")
            .default_model
            .expect("minimax default model");
        assert!(ids.contains(&def), "default model not in the doc list: {}", def);
        // 国内站同一份清单（同一套模型线）。
        assert_eq!(
            preset_models("minimax-cn").map(|m| m.len()),
            Some(models.len())
        );
        // 出处必须是官方文档地址（不许「来源不明」）。
        let doc = preset_models_doc("minimax").expect("doc source");
        assert!(
            doc.starts_with("https://platform.minimaxi.com/"),
            "not an official doc url: {}",
            doc
        );
    }

    /// ⚠️ **已下线的模型不许进清单** —— MiniMax 官方「模型列表」页把
    /// M2.x / music-* 列在「**历史模型**」里 ⇒ ⇒ 抄进清单就是「拿死模型
    /// 当默认」的老毛病 ⇒ ⇒ 负控钉住。
    #[test]
    fn retired_models_never_enter_the_doc_list() {
        let models = preset_models("minimax").expect("minimax doc list");
        for m in models {
            assert!(!m.id.contains("M2"), "retired model in list: {}", m.id);
            assert!(
                !m.id.to_lowercase().contains("music"),
                "retired model in list: {}",
                m.id
            );
            assert!(!m.id.contains("abab"), "retired model in list: {}", m.id);
        }
    }

    /// ⚠️ **没登记的预设就说「没有」** —— 清单不许瞎填（编出来的 id
    /// 会让用户撞「模型不存在」）⇒ ⇒ `None` 是合法答案。
    #[test]
    fn a_preset_without_a_doc_list_says_so() {
        assert!(preset_models("openai").is_none());
        assert!(preset_models("nope").is_none());
        assert!(preset_models_doc("openai").is_none());
        // 登记过的那几条，出处与清单都要在。
        for id in ["minimax", "minimax-cn", "gemini", "qianfan"] {
            assert!(preset_models(id).is_some(), "{} lost its doc list", id);
            assert!(preset_models_doc(id).is_some(), "{} lost its doc source", id);
        }
    }

    /// ⚠️ **没有任何预设还指着退役的老域**（`api.minimax.chat` /
    /// `api.minimaxi.chat`）—— 钉住「收老域」这个动作本身，
    /// 免得下一个刀又把老域抄回来。
    #[test]
    fn no_preset_points_at_a_retired_minimax_domain() {
        for p in presets() {
            assert!(
                !p.base_url.contains("api.minimax.chat")
                    && !p.base_url.contains("api.minimaxi.chat"),
                "{} still points at a retired MiniMax domain: {}",
                p.id,
                p.base_url
            );
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

    #[test]
    fn capability_inference_names_the_modality_or_stays_silent() {
        // 名字自己说清模态的那一族 ⇒ 标注。
        assert_eq!(infer_capability("tts-1"), Some("tts"));
        assert_eq!(infer_capability("whisper-1"), Some("asr"));
        assert_eq!(infer_capability("text-embedding-3-small"), Some("embedding"));
        assert_eq!(infer_capability("BAAI/bge-reranker-v2-m3"), Some("rerank"));
        assert_eq!(infer_capability("BAAI/bge-m3"), Some("embedding"));
        assert_eq!(infer_capability("dall-e-3"), Some("image_gen"));
        assert_eq!(
            infer_capability("black-forest-labs/FLUX.1-schnell"),
            Some("image_gen")
        );
        // ⚠️ **MiniMax H3 三兄弟**（2026-11-12）：都得出图；
        // 而视觉理解类 id（带 image 但不带 h3-image）**不许**误标成出图。
        assert_eq!(infer_capability("MiniMax-H3-Image"), Some("image_gen"));
        assert_eq!(
            infer_capability("MiniMax-H3-Image-Highspeed"),
            Some("image_gen")
        );
        assert_eq!(
            infer_capability("MiniMax-H3-Image-Context-IR"),
            Some("image_gen")
        );
        assert_eq!(infer_capability("minimax/h3-image"), Some("image_gen"));
        assert_eq!(infer_capability("some-vl-image-understanding"), None);
        assert_eq!(infer_capability("MiniMax/video-01"), Some("video_gen"));
        // ⚠️ **族缀 `-asr`**（2026-11-14，真机小米 MiMo）：以前落 `None`
        // ⇒ 兜底 `llm` ⇒ 分档卡 asr 档永远看不见；`-tts` 那半边一直认得。
        // ⚠️ **真 id 带 `v`**（2026-11-15 订正）：官方目录里是
        // `mimo-v2.5-asr` / `mimo-v2.5-tts` ⇒ ⇒ 以前写的 `mimo-2.5-asr`
        // 是**假 id** ⇒ ⇒ 这条判据绿成假象（真 id 从来没被它碰过）。
        assert_eq!(infer_capability("mimo-v2.5-asr"), Some("asr"));
        assert_eq!(infer_capability("mimo-v2.5-tts"), Some("tts"));
        assert_eq!(infer_capability("qwen3-asr-flash"), Some("asr"));
        assert_eq!(infer_capability("fun-asr"), Some("asr"));
        // MiMo 的**对话**兄弟不许被 asr/tts 词缀误吃（真 id）。
        assert_eq!(infer_capability("mimo-v2.6-pro"), None);
        assert_eq!(infer_capability("mimo-v2.6-flash"), None);
        assert_eq!(infer_capability("mimo-v2.6-pro-ultraspeed"), None);
    }

    /// ⚠️ **MiniMax 的语音/视频真 id**（2026-11-15）—— TTS 真 id 名字里
    /// 没有 `tts` ⇒ ⇒ 以前落 `None` ⇒ ⇒ 兜底成 `llm`。
    #[test]
    fn minimax_voice_and_video_ids_are_recognized() {
        assert_eq!(infer_capability("speech-2.8-hd"), Some("tts"));
        assert_eq!(infer_capability("speech-02-turbo"), Some("tts"));
        assert_eq!(infer_capability("asr-1.0"), Some("asr"));
        // 视频族：与 seedance/kling 同归 video_gen（端脑 2026-11-16 起有
        // 第 10 档 `video_gen` ⇒ ⇒ 不再归一成 image_gen）。
        assert_eq!(infer_capability("MiniMax-Hailuo-2.3"), Some("video_gen"));
        // 负控：对话族不许被 speech-/hailuo- 误吃。
        assert_eq!(infer_capability("MiniMax-M3.1-Flash"), None);
        assert_eq!(infer_capability("MiniMax-M3"), None);
        // 认不出 ⇒ None —— 聊天模型一个都不许误标。
        assert_eq!(infer_capability("gpt-4o"), None);
        assert_eq!(infer_capability("deepseek-ai/DeepSeek-V3"), None);
        assert_eq!(infer_capability("claude-sonnet-4-20250514"), None);
        assert_eq!(infer_capability("qwen-max"), None);
    }

    /// ⚠️ **小米 MiMo 的默认模型不能是死模型**（2026-11-15）：官方文档站
    /// 全站横幅写明 `MiMo-V2.5-Pro` / `MiMo-V2.5` 已于 2026.10.21
    /// 10:00(UTC+8) 正式下线 ⇒ ⇒ 预设拿死模型当默认 ⇒ ⇒ 用户新建完供应商，
    /// 第一次调用就撞「模型不存在」⇒ ⇒ 「配得出来、用不起来」。
    #[test]
    fn the_mimo_preset_points_at_a_live_model() {
        let p = find_preset("xiaomi-mimo").expect("xiaomi-mimo preset");
        assert_eq!(p.base_url, "https://api.xiaomimimo.com/v1");
        let model = p.default_model.expect("mimo preset default model");
        assert_eq!(model, "mimo-v2.6-pro");
        // 死模型不许再回来。
        assert_ne!(model, "mimo-v2.5-pro");
        assert_ne!(model, "mimo-v2.5");
    }

    #[test]
    fn openai_model_lists_carry_inferred_capability() {
        let body = r#"{
            "data": [
                {"id": "gpt-4o"},
                {"id": "dall-e-3"},
                {"id": "tts-1"},
                {"id": "whisper-1"}
            ]
        }"#;
        let models = parse_openai_models(body).unwrap();
        assert_eq!(models[0].capability.as_deref(), None);
        assert_eq!(models[1].capability.as_deref(), Some("image_gen"));
        assert_eq!(models[2].capability.as_deref(), Some("tts"));
        assert_eq!(models[3].capability.as_deref(), Some("asr"));
    }

    #[test]
    fn claude_model_lists_stay_conservative() {
        let body = r#"{"data": [{"id": "claude-sonnet-4-20250514"}]}"#;
        let models = parse_claude_models(body).unwrap();
        assert_eq!(models[0].capability.as_deref(), None);
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

    /// ⚠️ **真上游夹具**（2026-11-16）—— 这段 JSON 是**从 OpenRouter
    /// 的真响应里摘的**（`GET https://openrouter.ai/api/v1/models`，
    /// 2026-11-16 取）⇒ ⇒ 钉的是「上游的 `architecture.input_modalities`
    /// 真会被读」⇒ ⇒ **不用自造形状**（自造的形状会把「上游换了字段名」
    /// 藏起来）。
    ///
    /// ⚠️ **来源要说对**：上游说 text,image,file ⇒ ⇒ 归一后
    /// text,image,**pdf** ⇒ ⇒ 而**出厂清单**对 claude/gpt-4o 只说
    /// text,image（**少一档**）⇒ ⇒ 两边**不一样** ⇒ ⇒ 这条判据**有牙**
    /// （要是读的是出厂清单就红）。
    #[test]
    fn the_real_openrouter_modalities_win_over_the_catalog() {
        let body = r#"{"data":[
{"id":"google/gemini-4-hydrogen","context_length":1048576,"architecture":{"input_modalities":["audio","file","image","text","video"],"output_modalities":["text"],"modality":"text+image+file+audio+video->text"}},
{"id":"anthropic/claude-sonnet-4.5","context_length":1000000,"architecture":{"input_modalities":["text","image","file"],"output_modalities":["text"]}},
{"id":"openai/gpt-4o-mini","context_length":128000,"architecture":{"input_modalities":["text","image","file"],"output_modalities":["text"]}},
{"id":"totally-unknown-model-xyz","context_length":1000}
]}"#;
        let models = parse_openai_models(body).expect("真上游那段的形状要能解析");
        assert_eq!(models.len(), 4);
        // ❶ Gemini 4：上游五档 ⇒ ⇒ 归一后 **file ⇒ pdf** ⇒ ⇒ 顺序照上游
        // （出厂清单那份的顺序不一样 ⇒ ⇒ 顺序本身就是判据）。
        let g = &models[0];
        assert_eq!(g.id, "google/gemini-4-hydrogen");
        assert_eq!(
            g.input_modalities,
            vec!["audio", "pdf", "image", "text", "video"]
        );
        assert_eq!(g.input_modalities_source.as_deref(), Some("upstream"));
        // ❷ Claude：上游 text,image,file ⇒ ⇒ **pdf**（出厂清单只说 text,image）。
        let c = &models[1];
        assert_eq!(c.input_modalities, vec!["text", "image", "pdf"]);
        assert_eq!(c.input_modalities_source.as_deref(), Some("upstream"));
        // ❸ gpt-4o-mini：同上（出厂清单只说 text,image ⇒ 少 pdf）。
        let o = &models[2];
        assert_eq!(o.input_modalities, vec!["text", "image", "pdf"]);
        assert_eq!(o.input_modalities_source.as_deref(), Some("upstream"));
        // ❹ 上游没说 ＋ 出厂清单也不认识 ⇒ ⇒ **留空 ＝ 不知道**
        // （**不是**「只能吃文本」⇒ ⇒ 界面不许瞎画）。
        let u = &models[3];
        assert!(u.input_modalities.is_empty());
        assert_eq!(u.input_modalities_source, None);
    }

    /// ⚠️ **上游说了不算数的词 ⇒ ⇒ 走出厂清单**（2026-11-16）——
    /// 上游可能把「能力」混进模态字段（`tools` / `function_calling`）
    /// ⇒ ⇒ 认不出的词**不算模态**⇒ ⇒ 不许拿它当「上游真给了」。
    #[test]
    fn an_upstream_word_we_do_not_know_falls_back_to_the_catalog() {
        let body = r#"{"data":[
{"id":"anthropic/claude-sonnet-4.5","architecture":{"input_modalities":["tools"]}},
{"id":"totally-unknown-model-xyz","architecture":{"input_modalities":["tools"]}}
]}"#;
        let models = parse_openai_models(body).unwrap();
        // ❶ 出厂清单认得 ⇒ ⇒ 走清单（来源 `catalog`）。
        assert_eq!(models[0].input_modalities, vec!["text", "image"]);
        assert_eq!(models[0].input_modalities_source.as_deref(), Some("catalog"));
        // ❷ 谁都不认得 ⇒ ⇒ **留空**（不填兜底 `text`）。
        assert!(models[1].input_modalities.is_empty());
        assert_eq!(models[1].input_modalities_source, None);
    }

    /// ⚠️ **认厂商：主机名优先，预设 id 兜底**（2026-11-16）—— 这份清单说的
    /// 是**这家上游**有哪些模型 ⇒ ⇒ 归属由主机名说了算 ⇒ ⇒ **自定义条目**只要
    /// 指向官方域也认得（否则「自己手填 `api.minimax.cn` 的那条」永远拿不到
    /// 出图／视频／语音那几档）。
    #[test]
    fn the_vendor_is_recognized_by_the_host_name_first() {
        // ❶ 官方域 ⇒ 认得（**没有 preset_id 也认**）。
        let cn = preset_doc_list(None, Some("https://api.minimax.cn/v1")).expect("minimax cn");
        assert_eq!(cn.preset_id, "minimax-cn");
        // ❷ 国际域。
        let io = preset_doc_list(None, Some("https://api.minimax.io/v1")).expect("minimax io");
        assert_eq!(io.preset_id, "minimax");
        // ❸ **主机名认不出**（自建代理）⇒ ⇒ 退回**预设 id**。
        let proxy = preset_doc_list(Some("minimax-cn"), Some("http://127.0.0.1:8080/v1"))
            .expect("fallback to preset id");
        assert_eq!(proxy.preset_id, "minimax-cn");
        // ❹ 两样都没有 ⇒ ⇒ 没有清单（**不糊弄**）。
        assert!(preset_doc_list(None, Some("http://127.0.0.1:8080/v1")).is_none());
        // ❺ 老域**也认**（存量条目还指着它 ⇒ ⇒ 它仍是同一家厂商）。
        let legacy =
            preset_doc_list(None, Some("https://api.minimax.chat/v1")).expect("legacy host");
        assert_eq!(legacy.preset_id, "minimax");
    }

    /// ⚠️ **主机名优先于预设 id**（2026-11-16）—— 「用户挑了 Gemini 预设、却把
    /// 地址改成 MiniMax」⇒ ⇒ 清单必须是 **MiniMax 的** ⇒ ⇒ 不把 A 家的模型
    /// 抄给 B 家。
    #[test]
    fn the_host_name_wins_over_the_preset_id() {
        let list = preset_doc_list(Some("gemini"), Some("https://api.minimax.cn/v1"))
            .expect("host wins");
        assert_eq!(list.preset_id, "minimax-cn");
        assert!(list.models.iter().any(|m| m.id == "MiniMax-H3-Image"));
        assert!(!list.models.iter().any(|m| m.id.starts_with("gemini-")));
    }

    /// ⚠️ **上游没列的条目**（2026-11-16）—— MiniMax 的 `GET /v1/models` 逐字
    /// 只给两条语言模型 ⇒ ⇒ 出图／视频／语音那几档**上游没列** ⇒ ⇒ 读面要拿
    /// 它们**补齐** ⇒ ⇒ 补的**按 id 算**（上游列过的**不重复**）。
    #[test]
    fn the_models_the_upstream_did_not_list_are_computed_by_id() {
        let list = preset_doc_list(Some("minimax-cn"), Some("https://api.minimax.cn/v1"))
            .expect("minimax cn list");
        // ❶ 上游逐字只给的两条语言模型。
        let upstream = vec!["MiniMax-M3.1-Flash".to_string(), "MiniMax-M3".to_string()];
        let missing = preset_models_missing(list, &upstream);
        let ids: Vec<&str> = missing.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"MiniMax-H3-Image"), "{ids:?}");
        assert!(ids.contains(&"speech-2.8-hd"), "{ids:?}");
        // ❷ 上游已经列过的**不重复**。
        assert!(!ids.contains(&"MiniMax-M3.1-Flash"), "{ids:?}");
        assert!(!ids.contains(&"MiniMax-M3"), "{ids:?}");
        // ❸ 上游**也列了**出图模型 ⇒ ⇒ 那条**不补**（上游说了算）。
        let upstream2 = vec![
            "MiniMax-M3.1-Flash".to_string(),
            "MiniMax-H3-Image".to_string(),
        ];
        let missing2 = preset_models_missing(list, &upstream2);
        assert!(!missing2.iter().any(|m| m.id == "MiniMax-H3-Image"));
    }

    /// ⚠️ **主机名怎么读**（2026-11-16）—— 只认主机名（路径、端口、大小写都不
    /// 影响）⇒ ⇒ 少写一个 `/v1` 不会失配 ⇒ ⇒ 认错了就是「认不出 ⇒ ⇒ 不补」。
    #[test]
    fn the_host_name_is_read_without_the_path_or_the_port() {
        assert_eq!(
            base_host("https://api.minimax.cn/v1").as_deref(),
            Some("api.minimax.cn")
        );
        assert_eq!(
            base_host("api.minimax.cn/v1").as_deref(),
            Some("api.minimax.cn")
        );
        assert_eq!(
            base_host("https://api.minimax.cn:8443/v1").as_deref(),
            Some("api.minimax.cn")
        );
        assert_eq!(
            base_host("HTTPS://API.MINIMAX.CN/v1").as_deref(),
            Some("api.minimax.cn")
        );
        assert!(base_host("").is_none());
    }
}
