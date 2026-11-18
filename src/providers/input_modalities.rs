//! **出厂「模型 → 输入模态」清单** —— 兜底用的产品知识（随版本走）。
//!
//! ## 为什么要有这个文件
//!
//! ⚠️ 云端供应商的模型清单接口（OpenAI 兼容面）**多数只回 `id`** ⇒ ⇒ 拉回来
//! 的模型**看不出支持什么输入格式** ⇒ ⇒ 客户端只能自己猜 ⇒ ⇒ 两端各猜一套
//! ⇒ ⇒ 「界面说能传图、实际传了报 400」。
//!
//! ## 与「能力档」是**两张表**（important）
//!
//! - `model_capabilities.rs` 判的是**能力档**（`llm` / `asr` / `vision` …）
//!   ⇒ ⇒ 「这条模型是哪一类」；
//! - **本表**判的是**输入模态**（`text` / `image` / `audio` / `video` / `pdf`）
//!   ⇒ ⇒ 「这条模型吃什么」。
//!
//! ⇒ ⇒ 两个轴**不许混进一张表**（一条 `asr` 模型的能力档是 `asr`，但它的输入
//! 模态是 `audio`；一条 `llm` 模型的能力档可以是 `vision`，输入模态却可能只有
//! `text`）。
//!
//! ## 取证纪律（⚠️ 只写文档里真有的）
//!
//! 每条规则都带**官方文档地址** ⇒ ⇒ 界面能说「这是按哪份文档预设的」
//! ⇒ ⇒ 也能被人**逐条复核**。
//!
//! ⚠️ **取不到官方文档逐字的 ⇒ 不登记**（宁可没有，不编）：
//! - `deepseek-chat` / `deepseek-reasoner` —— 官方文档站（`api-docs.deepseek.com`）
//!   在本机取到的页面**逐字里没有模型名**（JS 壳页）⇒ ⇒ **不登记**
//!   ⇒ ⇒ 它们落到兜底 `text`（而这恰好是对的）。
//!
//! ## 匹配语义
//!
//! 与能力档那张表**同一种语义**（见 `glob::matches`）：模式用 `*` 包裹 ⇒ ⇒
//! **子串出现即命中**（模型 id 带命名空间前缀是常态）。
//!
//! ⚠️ **规则按 `pattern` 长度降序求值**（最具体的先试）—— 否则
//! `*gpt-4o*`（→ text/image）会吃掉 `*gpt-4o-transcribe*`（→ audio）。

use super::glob::matches;

/// **输入模态词表** —— 两端共用的**唯一**词表。
///
/// ⚠️ `pdf` = 文档类输入（PDF / 文件）。上游若只说 `file` ⇒ 归到 `pdf`
/// （本词表里唯一的文档类）。
pub const INPUT_MODALITIES: &[&str] = &["text", "image", "audio", "video", "pdf"];

/// 一条「模型 id 模式 → 输入模态」规则。
#[derive(Clone, Copy)]
pub struct InputModalityRule {
    /// 小写模式（`*` = 任意串）。
    pub pattern: &'static str,
    /// 词表里的模态词（`INPUT_MODALITIES` 的子集）。
    pub modalities: &'static [&'static str],
    /// **官方文档地址** —— 每条规则都必须带（可逐条复核）。
    pub doc: &'static str,
}

const fn rule(
    pattern: &'static str,
    modalities: &'static [&'static str],
    doc: &'static str,
) -> InputModalityRule {
    InputModalityRule {
        pattern,
        modalities,
        doc,
    }
}

// ── 官方文档地址（逐条规则引用同一家的同一页）──────────────────────────
const DOC_OPENAI_GPT4O: &str = "https://developers.openai.com/api/docs/models/gpt-4o";
const DOC_OPENAI_STT: &str = "https://platform.openai.com/docs/guides/speech-to-text";
const DOC_OPENAI_TTS: &str = "https://platform.openai.com/docs/guides/text-to-speech";
const DOC_ANTHROPIC_VISION: &str = "https://platform.claude.com/docs/en/build-with-claude/vision.md";
const DOC_GEMINI: &str = "https://ai.google.dev/gemini-api/docs/models/gemini";
const DOC_GEMINI_IMAGE: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-3-pro-image";
const DOC_GEMINI_3_PRO: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-3-pro-preview";
const DOC_GEMINI_3_FLASH: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-3-flash-preview";
const DOC_GEMINI_25_PRO: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-2.5-pro";
const DOC_GEMINI_20_FLASH: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-2.0-flash";
const DOC_GEMINI_OMNI: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-omni-flash";
const DOC_GEMINI_25_FLASH: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash";
const DOC_GEMINI_25_FLASH_LITE: &str = "https://ai.google.dev/gemini-api/docs/models/gemini-2.5-flash-lite";
const DOC_XAI: &str = "https://docs.x.ai/docs/models";
const DOC_QWEN_OMNI: &str = "https://help.aliyun.com/zh/model-studio/qwen-omni";
const DOC_QWEN_VISION: &str = "https://help.aliyun.com/zh/model-studio/vision";
const DOC_KIMI_VISION: &str = "https://platform.moonshot.cn/docs/guide/use-kimi-vision-model.md";
const DOC_MINIMAX_TEXT: &str = "https://platform.minimaxi.com/docs/guides/text-generation.md";
const DOC_MINIMAX_IMAGE: &str = "https://platform.minimaxi.com/docs/guides/image-generation.md";
const DOC_MINIMAX_TTS: &str = "https://platform.minimaxi.com/docs/guides/speech-t2a-websocket.md";
const DOC_MINIMAX_ASR: &str = "https://platform.minimaxi.com/docs/guides/speech-to-text.md";
const DOC_MISTRAL: &str = "https://docs.mistral.ai/getting-started/models/models_overview/";

/// 声明顺序**不重要**（运行时按长度降序）；分七段读：OpenAI → Anthropic →
/// Google → xAI → 阿里百炼 → Kimi → MiniMax → Mistral。
pub static RULES: &[InputModalityRule] = &[
    // ── OpenAI ──────────────────────────────────────────────────────────
    // ⚠️ **转写族吃音频**（官方 speech-to-text 页逐字列出 `whisper-1` /
    // `gpt-4o-transcribe` / `gpt-4o-mini-transcribe` / `gpt-4o-transcribe-diarize`）
    // ⇒ ⇒ 模式必须比 `*gpt-4o*` 长 ⇒ ⇒ 否则被它吃掉落 text/image ✗。
    rule("*gpt-4o-transcribe*", &["audio"], DOC_OPENAI_STT),
    rule("*whisper-*", &["audio"], DOC_OPENAI_STT),
    // ⚠️ **合成族吃文本**（官方 text-to-speech 页逐字列出 `tts-1` / `tts-1-hd` /
    // `gpt-4o-mini-tts`）⇒ ⇒ 输入是文本（输出才是音频）。
    rule("*gpt-4o-mini-tts*", &["text"], DOC_OPENAI_TTS),
    rule("tts-1*", &["text"], DOC_OPENAI_TTS),
    // ⚠️ **GPT-4o 吃文本 + 图片**（官方模型卡逐字：Modalities `Text, Image`）。
    rule("*gpt-4o*", &["text", "image"], DOC_OPENAI_GPT4O),
    // ── Anthropic ───────────────────────────────────────────────────────
    // ⚠️ **可核性说明**：该页在本机**区域被屏蔽**（页面逐字是
    // "App unavailable in region"）⇒ ⇒ 但它的**地址**由 Anthropic 官方
    // `llms.txt` 索引逐字列出（`.../build-with-claude/vision.md`）
    // ⇒ ⇒ 按地址登记，并在台账里注明这一点。
    rule("*claude-*", &["text", "image"], DOC_ANTHROPIC_VISION),
    // ── Google ──────────────────────────────────────────────────────────
    // ⚠️ **图像模型吃文本 + 图片**（官方 gemini-3-pro-image 页逐字：
    // "**Inputs** Image and Text"）。
    rule("*gemini*image*", &["text", "image"], DOC_GEMINI_IMAGE),
    // ⚠️ **Gemini 4 族吃五种**（官方 Gemini 模型页逐字：
    // "**Modalities** **Text, Image, Audio, Video, PDF**"）。
    // ⚠️ 模式必须比 `*gemini*image*` 短没关系（前者更长 ⇒ 先试）——
    // 两条互不重叠（`gemini-4-carbon` 不含 "image"）。
    rule("*gemini-4*", &["text", "image", "audio", "video", "pdf"], DOC_GEMINI),
    // ⚠️ **Gemini 3.x 族**（2026-11-16 补登记）—— 官方**每模型页**逐字
    // 「**Inputs** Text, Image, Video, Audio, and PDF」（gemini-3-pro-preview 与
    // gemini-3-flash-preview 两页逐字相同）⇒ ⇒ 五个模态。
    rule(
        "*gemini-3-pro-preview*",
        &["text", "image", "video", "audio", "pdf"],
        DOC_GEMINI_3_PRO,
    ),
    rule(
        "*gemini-3-flash-preview*",
        &["text", "image", "video", "audio", "pdf"],
        DOC_GEMINI_3_FLASH,
    ),
    // ⚠️ **Gemini 2.5 族：官方逐字**逐模型不一样**（这一族不许用一条通配）——
    //   * gemini-2.5-pro        ⇒ 「**Inputs** Audio, images, video, text, and PDF」
    //   * gemini-2.5-flash      ⇒ 「**Inputs** Text, images, video, audio」（**没有 PDF**）
    //   * gemini-2.5-flash-lite ⇒ 「**Inputs** Text, image, video, audio, PDF」
    // ⇒ ⇒ 逐条分开登记；长度降序 ⇒ 更长的（flash-lite）先试。
    rule(
        "*gemini-2.5-flash-lite*",
        &["text", "image", "video", "audio", "pdf"],
        DOC_GEMINI_25_FLASH_LITE,
    ),
    rule(
        "*gemini-2.5-flash*",
        &["text", "image", "video", "audio"],
        DOC_GEMINI_25_FLASH,
    ),
    rule(
        "*gemini-2.5-pro*",
        &["text", "image", "video", "audio", "pdf"],
        DOC_GEMINI_25_PRO,
    ),
    // ⚠️ **Gemini 2.0 Flash**（官方逐字：「**Inputs** Audio, images, video, and
    // text」⇒ ⇒ **没有 PDF**）⇒ ⇒ 与 2.5 族一样逐条登记。
    rule(
        "*gemini-2.0-flash*",
        &["text", "image", "video", "audio"],
        DOC_GEMINI_20_FLASH,
    ),
    // ⚠️ **Gemini omni 族**（官方逐字：「**Input** Text, Image, Video（编辑最长
    // 15s、延展回看 10s）」⇒ ⇒ **没有音频、没有 PDF**）⇒ ⇒ 逐字登记。
    rule(
        "*gemini-omni*",
        &["text", "image", "video"],
        DOC_GEMINI_OMNI,
    ),
    // ── xAI ─────────────────────────────────────────────────────────────
    // ⚠️ 官方模型页**内嵌** `__XAI_PUBLIC_MODELS__` JSON ⇒ ⇒ 每个模型的
    // `inputModalities` 都是机器可读的逐字值（本表按它抄）。
    // ⚠️ **变体不一样**：`grok-imagine-video-1.5` 是 TEXT/IMAGE/AUDIO、
    // `grok-imagine-video-1.5-lite` 是 TEXT/IMAGE、`grok-imagine-video` 才是
    // TEXT/IMAGE/VIDEO ⇒ ⇒ 三条**分开**登记（按长度降序求值）。
    rule(
        "*grok-imagine-video-1.5-lite*",
        &["text", "image"],
        DOC_XAI,
    ),
    rule(
        "*grok-imagine-video-1.5*",
        &["text", "image", "audio"],
        DOC_XAI,
    ),
    rule("*grok-imagine-video*", &["text", "image", "video"], DOC_XAI),
    // ⚠️ 转写族只吃音频（JSON 逐字：`"grok-voice-transcribe-2.0" => "AUDIO"`）。
    rule("*grok-voice-transcribe*", &["audio"], DOC_XAI),
    // ⚠️ 语音对话族吃文本 + 音频（JSON 逐字：`"grok-voice-2.0" => "TEXT","AUDIO"`）。
    rule("*grok-voice*", &["text", "audio"], DOC_XAI),
    // ⚠️ 合成族吃文本（JSON 逐字：`"grok-tts" => "TEXT"`）。
    rule("*grok-tts*", &["text"], DOC_XAI),
    // ⚠️ Grok 主力族吃文本 + 图片（JSON 逐字：`"grok-4.3" => "TEXT","IMAGE"`）。
    rule("*grok*", &["text", "image"], DOC_XAI),
    // ── 阿里百炼（Qwen）──────────────────────────────────────────────────
    // ⚠️ **全模态族**（官方 qwen-omni 页逐字：「使用 Qwen-Omni **理解文本、
    // 图片、音频和视频**」）。
    rule("*qwen*omni*", &["text", "image", "audio", "video"], DOC_QWEN_OMNI),
    // ⚠️ **视觉理解族**（官方 vision 页逐字：「视觉理解模型可以根据您传入的
    // **图片或视频**进行回答，支持单图或多图的输入」）。
    rule("*qwen*vl*", &["text", "image", "video"], DOC_QWEN_VISION),
    // ── Kimi（月之暗面）──────────────────────────────────────────────────
    // ⚠️ 官方视觉模型指南逐字：「`kimi-k3.1-pro` 和 `kimi-k3` **支持文本、
    // 图片与视频输入**」。
    rule("*kimi-k3*", &["text", "image", "video"], DOC_KIMI_VISION),
    // ── MiniMax ─────────────────────────────────────────────────────────
    // ⚠️ **语言模型族吃文本 + 图片 + 视频**（官方 text-generation 页逐字：
    // 「多模态 Chat 输入 … **支持文本、图片和视频**等多模态输入」）。
    rule("*minimax-m3*", &["text", "image", "video"], DOC_MINIMAX_TEXT),
    // ⚠️ **出图模型**（官方 image-generation 页逐字：文生图 = prompt；
    // 图片编辑 = prompt + 1~16 张参考图）⇒ ⇒ 吃文本 + 图片。
    rule("*h3-image*", &["text", "image"], DOC_MINIMAX_IMAGE),
    // ⚠️ **语音合成**吃文本（官方 speech-t2a 页：按文本生成音频）。
    rule("*speech-*", &["text"], DOC_MINIMAX_TTS),
    // ⚠️ **语音识别**吃音频（官方 speech-to-text 页逐字：「MiniMax ASR 将
    // **音频**转写为文本」）。
    rule("*asr-*", &["audio"], DOC_MINIMAX_ASR),
    // ── Mistral ─────────────────────────────────────────────────────────
    // ⚠️ 官方模型总览页逐字：Pixtral Large「offering best-in-class **text and
    // vision** capabilities」；Mistral Small 同款描述。
    rule("*pixtral*", &["text", "image"], DOC_MISTRAL),
    rule("*mistral-small*", &["text", "image"], DOC_MISTRAL),
];

fn sorted_rules() -> &'static [InputModalityRule] {
    use std::sync::OnceLock;
    static SORTED: OnceLock<Vec<InputModalityRule>> = OnceLock::new();
    SORTED.get_or_init(|| {
        let mut v = RULES.to_vec();
        // 长度降序；同长度按字母序（确定性）。
        v.sort_by(|a, b| {
            b.pattern
                .len()
                .cmp(&a.pattern.len())
                .then_with(|| a.pattern.cmp(b.pattern))
        });
        v
    })
}

/// 查出厂清单（大小写不敏感）。
///
/// `None` ⇒ 清单不认识这条模型（**不是**「只能吃文本」）⇒ ⇒ 调用方继续走
/// 兜底 `text`。
pub fn catalog_input_modalities(model_id: &str) -> Option<&'static [&'static str]> {
    let id = model_id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    sorted_rules()
        .iter()
        .find(|r| matches(r.pattern, &id))
        .map(|r| r.modalities)
}

/// 这条模型的模态清单**出自哪份官方文档** —— 界面要能说「按哪份文档预设的」。
pub fn catalog_input_modalities_doc(model_id: &str) -> Option<&'static str> {
    let id = model_id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    sorted_rules()
        .iter()
        .find(|r| matches(r.pattern, &id))
        .map(|r| r.doc)
}

/// 上游给的模态词 → 词表里的词（大小写/别名归一）。
///
/// `None` ⇒ 不认识的词（`tools` / `function_calling` 这类**不是模态**的词
/// ⇒ ⇒ 跳过，不当模态）。
pub fn normalize_modality(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "text" | "txt" | "string" => Some("text"),
        "image" | "images" | "image_url" => Some("image"),
        "audio" | "sound" => Some("audio"),
        "video" => Some("video"),
        // ⚠️ `file` 归 `pdf` —— 本词表里唯一的文档类（OpenRouter 的
        // `architecture.input_modalities` 用的是 `file`）。
        "pdf" | "document" | "file" => Some("pdf"),
        _ => None,
    }
}

/// **上游给的输入模态原值** —— 三种已知形状里取**第一个非空的**。
///
/// ① `architecture.input_modalities`（**OpenRouter 的真形状**）
/// ② 顶层 `input_modalities`
/// ③ `modalities.input`
///
/// ⚠ 返的是**原值**（不归一）⇒ ⇒ 「归一 ＋ 判来源」是**读面**的活
/// （端脑读面 `effective_input_modalities`）⇒ ⇒ 这里只回答
/// 「**上游说了什么**」⇒ ⇒ **目录层与端脑写面共用这一条口径**
/// （别各写一套 ⇒ ⇒ 上游换个形状两处不会跑偏）。
pub fn pick_upstream_modalities(
    architecture: &[String],
    top_level: &[String],
    modalities_input: &[String],
) -> Vec<String> {
    for candidate in [architecture, top_level, modalities_input] {
        let list: Vec<String> = candidate
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    Vec::new()
}

/// 上游给的输入模态原值（从**原始 JSON** 里取）—— 端脑的写面用它。
///
/// ⚠ 形状判定与 `pick_upstream_modalities` **同一套**。
pub fn upstream_input_modalities(model: &serde_json::Value) -> Vec<String> {
    fn words(v: Option<&serde_json::Value>) -> Vec<String> {
        v.and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
    let architecture = words(model.pointer("/architecture/input_modalities"));
    let top_level = words(model.get("input_modalities"));
    let modalities_input = words(model.pointer("/modalities/input"));
    pick_upstream_modalities(&architecture, &top_level, &modalities_input)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ **夹具一律用官方文档里真有的 id**（出厂清单的判据必须用真 id 当夹具
    /// —— 写不存在的 id，判据会绿成假象）。
    #[test]
    fn the_documented_families_land_on_the_documented_modalities() {
        let cases: &[(&str, &[&str])] = &[
            // OpenAI（官方模型卡逐字：Modalities "Text, Image"）。
            ("gpt-4o", &["text", "image"]),
            ("gpt-4o-mini", &["text", "image"]),
            ("gpt-4o-transcribe", &["audio"]),
            ("gpt-4o-mini-tts", &["text"]),
            ("whisper-1", &["audio"]),
            // Anthropic。
            ("claude-sonnet-4", &["text", "image"]),
            // Google（官方逐字：Modalities "Text, Image, Audio, Video, PDF"）。
            ("gemini-4-carbon", &["text", "image", "audio", "video", "pdf"]),
            ("gemini-3-pro-image", &["text", "image"]),
            // ⚠️ **Gemini 2.5/3.x 逐模型不一样**（官方每模型页逐字）：
            //   2.5-flash **没有 PDF**，2.5-flash-lite / 2.5-pro / 3.x 有。
            ("gemini-2.5-pro", &["text", "image", "video", "audio", "pdf"]),
            ("gemini-2.5-flash", &["text", "image", "video", "audio"]),
            ("gemini-2.5-flash-lite", &["text", "image", "video", "audio", "pdf"]),
            ("gemini-2.0-flash", &["text", "image", "video", "audio"]),
            ("gemini-3-pro-preview", &["text", "image", "video", "audio", "pdf"]),
            ("gemini-3-flash-preview", &["text", "image", "video", "audio", "pdf"]),
            ("gemini-omni-flash", &["text", "image", "video"]),
            // xAI（官方页内嵌 JSON 的 inputModalities）。
            ("grok-4.3", &["text", "image"]),
            ("grok-voice-2.0", &["text", "audio"]),
            ("grok-voice-transcribe-2.0", &["audio"]),
            ("grok-tts", &["text"]),
            ("grok-imagine-video", &["text", "image", "video"]),
            ("grok-imagine-video-1.5", &["text", "image", "audio"]),
            ("grok-imagine-video-1.5-lite", &["text", "image"]),
            // 阿里百炼。
            ("qwen3-omni-flash", &["text", "image", "audio", "video"]),
            ("qwen-vl-max", &["text", "image", "video"]),
            // Kimi。
            ("kimi-k3.1-pro", &["text", "image", "video"]),
            // MiniMax。
            ("MiniMax-M3.1-Flash", &["text", "image", "video"]),
            ("MiniMax-H3-Image", &["text", "image"]),
            ("speech-2.8-hd", &["text"]),
            ("asr-1.0", &["audio"]),
            // Mistral。
            ("pixtral-large-24-11", &["text", "image"]),
            ("mistral-small-4-0", &["text", "image"]),
        ];
        for (id, expected) in cases {
            assert_eq!(
                catalog_input_modalities(id),
                Some(*expected),
                "input modalities of {id}"
            );
        }
    }

    /// ⚠️ **更具体的模式先命中**（不是声明顺序）—— 与能力档那张表同一陷阱
    /// （对话族前缀不许吃掉更具体的语音/转写族）。
    #[test]
    fn the_more_specific_pattern_wins() {
        // gpt-4o（text/image）不许吃掉 gpt-4o-transcribe（audio）。
        assert_eq!(catalog_input_modalities("gpt-4o-transcribe"), Some(&["audio"][..]));
        assert_eq!(catalog_input_modalities("gpt-4o"), Some(&["text", "image"][..]));
        // grok-voice（text/audio）不许吃掉 grok-voice-transcribe（audio）。
        assert_eq!(
            catalog_input_modalities("grok-voice-transcribe-2.0"),
            Some(&["audio"][..])
        );
        // grok-imagine-video（text/image/video）不许吃掉 1.5 变体。
        assert_eq!(
            catalog_input_modalities("grok-imagine-video-1.5"),
            Some(&["text", "image", "audio"][..])
        );
        // ⚠️ **Gemini 2.5 族逐模型不一样** ⇒ ⇒ flash-lite（有 PDF）不许被
        // flash（**没有** PDF）吃掉。
        assert_eq!(
            catalog_input_modalities("gemini-2.5-flash-lite"),
            Some(&["text", "image", "video", "audio", "pdf"][..])
        );
        assert_eq!(
            catalog_input_modalities("gemini-2.5-flash"),
            Some(&["text", "image", "video", "audio"][..])
        );
        // asr-* 不许被语音合成族吃掉。
        assert_eq!(catalog_input_modalities("asr-1.0"), Some(&["audio"][..]));
        assert_eq!(catalog_input_modalities("speech-2.8-hd"), Some(&["text"][..]));
    }

    /// ⚠️ **没登记的 ⇒ 不许编**（DeepSeek 的官方文档在本机取不到逐字 ⇒ ⇒
    /// 不登记 ⇒ ⇒ 落兜底 text —— 而这恰好是对的那一边）。
    #[test]
    fn unregistered_models_stay_silent() {
        assert_eq!(catalog_input_modalities("deepseek-chat"), None);
        assert_eq!(catalog_input_modalities("deepseek-reasoner"), None);
        assert_eq!(catalog_input_modalities("totally-unknown-model-xyz"), None);
        assert_eq!(catalog_input_modalities(""), None);
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(catalog_input_modalities("GPT-4O"), Some(&["text", "image"][..]));
        assert_eq!(
            catalog_input_modalities("MiniMax/MiniMax-M3.1-Flash"),
            Some(&["text", "image", "video"][..])
        );
    }

    /// ⚠️ **每条规则都得带官方文档地址** —— 没有出处 ⇒ 等于编。
    #[test]
    fn every_rule_carries_an_official_doc_url() {
        for r in RULES {
            assert!(
                r.doc.starts_with("https://"),
                "规则 {:?} 没有官方文档地址",
                r.pattern
            );
        }
    }

    /// ⚠️ **词表合法** —— 写错一个词 ⇒ 两端渲染不出来。
    #[test]
    fn every_modality_word_is_in_the_vocabulary() {
        for r in RULES {
            assert!(!r.modalities.is_empty(), "规则 {:?} 没有模态", r.pattern);
            for m in r.modalities {
                assert!(
                    INPUT_MODALITIES.contains(m),
                    "规则 {:?} 里的模态 {:?} 不在词表",
                    r.pattern,
                    m
                );
            }
        }
    }

    #[test]
    fn the_doc_lookup_agrees_with_the_modality_lookup() {
        assert_eq!(
            catalog_input_modalities_doc("grok-4.3"),
            Some(DOC_XAI),
            "xAI 那条必须指向官方模型页"
        );
        assert_eq!(
            catalog_input_modalities_doc("gemini-4-carbon"),
            Some(DOC_GEMINI),
            "Gemini 那条必须指向官方 Gemini 模型页"
        );
        assert_eq!(catalog_input_modalities_doc("totally-unknown-model-xyz"), None);
    }

    #[test]
    fn upstream_modality_words_are_normalized() {
        assert_eq!(normalize_modality("TEXT"), Some("text"));
        assert_eq!(normalize_modality("Image"), Some("image"));
        assert_eq!(normalize_modality(" audio "), Some("audio"));
        assert_eq!(normalize_modality("file"), Some("pdf"));
        // ⚠️ 不是模态的词 ⇒ 不许当成模态。
        assert_eq!(normalize_modality("tools"), None);
        assert_eq!(normalize_modality("function_calling"), None);
        assert_eq!(normalize_modality(""), None);
    }

    /// ⚠️ **上游的三种形状只有一套口径**（2026-11-16）—— 目录层与
    /// 端脑写面共用 `pick_upstream_modalities` ⇒ ⇒ 上游换个字段名两处
    /// 不会跑偏。
    #[test]
    fn the_upstream_shapes_are_picked_in_one_place() {
        // ① `architecture.input_modalities` 优先（OpenRouter 的真形状）。
        let all_three = serde_json::json!({
            "architecture": {"input_modalities": ["text", "image"]},
            "input_modalities": ["audio"],
            "modalities": {"input": ["video"]}
        });
        assert_eq!(
            upstream_input_modalities(&all_three),
            vec!["text".to_string(), "image".to_string()]
        );
        // ② 没有 architecture ⇒ ⇒ 顶层那份。
        let top = serde_json::json!({"input_modalities": ["audio"]});
        assert_eq!(upstream_input_modalities(&top), vec!["audio".to_string()]);
        // ③ 只有 `modalities.input` ⇒ ⇒ 那份。
        let nested = serde_json::json!({"modalities": {"input": ["video"]}});
        assert_eq!(upstream_input_modalities(&nested), vec!["video".to_string()]);
        // ④ 一个都没有 ⇒ ⇒ 空（**不是**「只能吃文本」）。
        assert!(upstream_input_modalities(&serde_json::json!({"id": "x"})).is_empty());
    }
}
