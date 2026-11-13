//! **出厂「模型 → 能力」清单** —— 兜底用的产品知识（随版本走）。
//!
//! ## 为什么要有这个文件
//!
//! ⚠️ 云端供应商的 `GET /v1/models`（OpenAI 兼容面）**只回 `id`**，不回
//! 能力标注 ⇒ 拉回来的模型能力全空 ⇒ 分档默认卡按词分桶时它们哪儿都进不去
//! ⇒ ⇒ 「获取了模型却没法按类选默认」。
//!
//! 端脑侧的判据链是**五档**（见 `telencephalon/src/api/providers.rs` 的
//! `effective_capability`）：
//! ```text
//! 用户覆盖 → 上游标注(非空) → 【本清单】 → 族缀推断 → llm
//! ```
//!
//! ## 与 `catalog::infer_capability` 的分工（important）
//!
//! 两者**不是替代关系，是补充关系**：
//! - `infer_capability`（`catalog.rs`）判的是**族缀**（`rerank` / `embedding` /
//!   `whisper` / `tts` / `dall-e` / `flux` …）—— 它覆盖「名字自己说清了模态」
//!   的那一族；
//! - **本清单补它覆盖不到的两类**：
//!   ① **语音交互族**（`*-audio-*` / `*-realtime-*` / `paraformer` /
//!      `sensevoice` / `cosyvoice` / `fish-speech`）—— 族缀表里没有这些词；
//!   ② **主流对话族**（`qwen*` / `deepseek*` / `glm-*` / `kimi*` …）——
//!      标出来是为了让「来源」显示成 `catalog`（**清单认得**）而不是
//!      `inferred`（**我们猜的**），用户据此判断哪条需要自己核对。
//!
//! ⇒ ⇒ **清单优先于推断**，所以清单里的每一条都必须**比推断更确定**；
//! 拿不准的**不要写进来**（留给推断或 `llm` 兜底，而不是写一条错的）。
//! 典型例子：`whisper-*` / `bge-*` / `dall-e-*` 归推断管，本表**不重复登记**。
//!
//! ## ⚠️ 词表边界
//!
//! 端脑 `Capability` 只有 9 档：`llm` / `asr` / `tts` / `vad` / `rerank` /
//! `embedding` / `vision` / `forecast` / `image_gen`。**没有 `video_gen`**
//! 也没有 `ocr` ⇒ ⇒ 视频生成族（`seedance` / `kling`）在本表归 `image_gen`；
//! 识别族（`ppocr`）**不登记**（留给推断/兜底，欠一条 ocr 词）。
//!
//! ## 匹配语义（⚠️ 为什么是「包含」而不是「前缀」）
//!
//! 模型 id 带命名空间前缀是**常态**（`vanchin/deepseek-v4.1-flash-1017`、
//! `kimi/kimi-k3.1-pro-highspeed`、`cmaas-glm-5.3`、`antdigital/Ling-…`）
//! ⇒ ⇒ 前缀匹配会漏掉一大半真实 id ⇒ ⇒ 本表**一律用 `*` 包裹**（子串出现
//! 即命中）。
//!
//! 代价与取舍：`*gpt-*` 也会命中 `my-gpt-4` 这类中转站别名 —— **那正是
//! 想要的行为**（它确实是 GPT 的转发）；反之一条**真正**的 `influx-node`
//! 会被 `*flux*` 误标 `image_gen`，但那种 id 不出现在对话上游的模型清单上
//! ⇒ 可接受。
//!
//! ⚠️ **规则按 `pattern` 长度降序求值**（最具体的先试）—— 否则
//! `*minimax*`（→ llm）会吃掉 `*minimax-h3*`（→ image_gen）。
//! 排序在运行时做（`sorted_rules`），声明顺序可以随时插；测试钉的是
//! **语义**（具体者先命中）与**词表合法**，不钉声明顺序。

/// 一条「模型 id 模式 → 能力词」规则。
#[derive(Clone, Copy)]
pub struct ModelCapabilityRule {
    /// 小写模式（`*` = 任意串）。
    pub pattern: &'static str,
    /// 端脑词表里的能力词（`Capability::ALL` 同一族拼法）。
    pub capability: &'static str,
}

const fn rule(pattern: &'static str, capability: &'static str) -> ModelCapabilityRule {
    ModelCapabilityRule {
        pattern,
        capability,
    }
}

/// 声明顺序**不重要**（运行时按长度降序）；分三段读：图像 → 语音 → 对话。
pub static RULES: &[ModelCapabilityRule] = &[
    // ── 图像 / 视频生成（最容易被对话族误吃）─────────────────────────
    rule("*stable-diffusion*", "image_gen"),
    rule("*minimax-h3*", "image_gen"),
    rule("*h3-image*", "image_gen"),
    rule("*seedream*", "image_gen"),
    rule("*cogview*", "image_gen"),
    rule("*seedance*", "image_gen"), // 端脑词表无 video 档 ⇒ 归 image_gen
    rule("*imagen*", "image_gen"),
    rule("*dall-e*", "image_gen"),
    rule("*kolors*", "image_gen"),
    rule("*kling*", "image_gen"),
    rule("*flux*", "image_gen"),
    // ── 语音（推断覆盖不到的一族）────────────────────────────────────
    rule("*audio-realtime*", "asr"),
    rule("*-realtime-*", "asr"),
    rule("*paraformer*", "asr"),
    rule("*sensevoice*", "asr"),
    rule("*cosyvoice*", "tts"),
    rule("*fish-speech*", "tts"),
    rule("*-audio-*", "asr"),
    rule("*audio*", "asr"),
    // ── 向量 / 重排（比对话族更具体，必须在前）────────────────────────
    rule("*text-embedding*", "embedding"),
    rule("*mistral-embed*", "embedding"),
    rule("*embedding-*", "embedding"),
    // ── 对话族（明确认得的主流名字 ⇒ 来源显示「清单」而非「推断」）─────
    rule("*command-r*", "llm"),
    rule("*sensechat*", "llm"),
    rule("*chatglm*", "llm"),
    rule("*deepseek*", "llm"),
    rule("*moonshot*", "llm"),
    rule("*baichuan*", "llm"),
    rule("*internlm*", "llm"),
    rule("*mixtral*", "llm"),
    rule("*mistral*", "llm"),
    rule("*hunyuan*", "llm"),
    rule("*minimax*", "llm"),
    rule("*doubao*", "llm"),
    rule("*gemini*", "llm"),
    rule("*claude*", "llm"),
    rule("*ernie*", "llm"),
    rule("*spark*", "llm"),
    rule("*llama*", "llm"),
    rule("*gemma*", "llm"),
    rule("*jamba*", "llm"),
    rule("*nova-*", "llm"),
    rule("*phi-*", "llm"),
    rule("*qwen*", "llm"),
    rule("*glm-*", "llm"),
    rule("*kimi*", "llm"),
    rule("*gpt-*", "llm"),
    rule("*grok*", "llm"),
    rule("*sonar*", "llm"),
    rule("*olmo*", "llm"),
    rule("*abab*", "llm"),
    rule("*step-*", "llm"),
    rule("*yi-*", "llm"),
    // 短名族：保留严格前缀（`o1*` 若写成 `*o1*` 会误吃 `proto1`）。
    rule("o1*", "llm"),
    rule("o3*", "llm"),
    rule("o4*", "llm"),
];

fn sorted_rules() -> &'static [ModelCapabilityRule] {
    use std::sync::OnceLock;
    static SORTED: OnceLock<Vec<ModelCapabilityRule>> = OnceLock::new();
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

/// glob 简化版：`*` 匹配任意串（顺序段匹配；首段也允许出现在任意位置
/// —— 这是「包含」语义，见文件头）。
fn matches(pattern: &str, id: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == id;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut cursor = 0usize;
    let leading_wildcard = pattern.starts_with('*');
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !leading_wildcard {
            // 无前导 `*` ⇒ 首段必须**前缀**命中（`o1*` 不许命中 `proto1`）。
            if !id[cursor..].starts_with(part) {
                return false;
            }
            cursor += part.len();
            continue;
        }
        match id[cursor..].find(part) {
            Some(pos) => cursor += pos + part.len(),
            None => return false,
        }
    }
    pattern.ends_with('*') || cursor == id.len()
}

/// 查出厂清单（大小写不敏感）。
///
/// `None` ⇒ 清单不认识这条模型（**不是**「没有能力」）⇒ 调用方继续走
/// 族缀推断，最后才落到 `llm`。
pub fn catalog_capability(model_id: &str) -> Option<&'static str> {
    let id = model_id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    sorted_rules()
        .iter()
        .find(|r| matches(r.pattern, &id))
        .map(|r| r.capability)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ **语义不变量**：更具体的模式先命中（不是声明顺序）。
    #[test]
    fn the_more_specific_pattern_wins() {
        // minimax 是对话族，但 H3 是出图模型。
        assert_eq!(catalog_capability("MiniMax-H3-Image"), Some("image_gen"));
        assert_eq!(catalog_capability("MiniMax/MiniMax-M3.1-Flash"), Some("llm"));
        // mistral 是对话族，但 mistral-embed 是向量模型（真机同类陷阱）。
        assert_eq!(catalog_capability("mistral-embed"), Some("embedding"));
        assert_eq!(catalog_capability("mistral-large-latest"), Some("llm"));
        // qwen 是对话族，但 qwen-audio 是语音模型。
        assert_eq!(
            catalog_capability("qwen-audio-3.1-realtime-flash"),
            Some("asr")
        );
        assert_eq!(catalog_capability("qwen3.8-max"), Some("llm"));
    }

    /// 这一族是 `infer_capability` **完全覆盖不到**的（真机踩到的那条）。
    #[test]
    fn audio_and_realtime_land_on_asr() {
        assert_eq!(catalog_capability("gpt-4o-audio-preview"), Some("asr"));
        assert_eq!(catalog_capability("gpt-4o-realtime-preview"), Some("asr"));
        assert_eq!(catalog_capability("paraformer-v2"), Some("asr"));
        assert_eq!(catalog_capability("cosyvoice-v2"), Some("tts"));
    }

    /// ⚠️ **命名空间前缀是常态** —— 这条测试就是真机那 182 条的形状。
    #[test]
    fn namespaced_ids_are_recognized() {
        assert_eq!(
            catalog_capability("vanchin/deepseek-v4.1-flash-1017"),
            Some("llm")
        );
        assert_eq!(
            catalog_capability("kimi/kimi-k3.1-pro-highspeed"),
            Some("llm")
        );
        assert_eq!(catalog_capability("cmaas-glm-5.3"), Some("llm"));
        assert_eq!(catalog_capability("antdigital/Ling-3.1-flash"), None);
    }

    #[test]
    fn mainstream_chat_families_are_recognized() {
        for id in [
            "gpt-4o",
            "claude-sonnet-4",
            "gemini-2.5-pro",
            "kimi-k3.1-pro",
            "doubao-pro-32k",
        ] {
            assert_eq!(catalog_capability(id), Some("llm"), "{id}");
        }
    }

    #[test]
    fn unknown_models_stay_silent() {
        assert_eq!(catalog_capability("totally-unknown-model-xyz"), None);
        assert_eq!(catalog_capability(""), None);
        // 短名族保留严格前缀：`o1*` 不许中间命中。
        assert_eq!(catalog_capability("proto1"), None);
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(catalog_capability("GPT-4O"), Some("llm"));
        // whisper 归推断管，本表不重复登记。
        assert_eq!(catalog_capability("Whisper-Large-V3"), None);
    }

    #[test]
    fn every_word_is_in_the_brain_vocabulary() {
        // ⚠️ 9 档词表（端脑 `Capability`）。写错一个词 ⇒ 端脑校验会拒，
        // 而清单是静态的 ⇒ 只在运行期才炸 ⇒ 这里先钉住。
        const VOCAB: &[&str] = &[
            "llm",
            "asr",
            "tts",
            "vad",
            "rerank",
            "embedding",
            "vision",
            "forecast",
            "image_gen",
        ];
        for r in RULES {
            assert!(
                VOCAB.contains(&r.capability),
                "清单里的词 {:?} 不在端脑词表（{}）",
                r.capability,
                r.pattern
            );
        }
    }
}
