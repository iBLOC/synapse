//! **glob 简化版匹配器** —— 出厂那两张表（能力档 / 输入模态）共用。
//!
//! ## 语义（⚠️ 为什么是「包含」而不是「前缀」）
//!
//! 模型 id 带命名空间前缀是**常态**（`vanchin/deepseek-v4.1-flash-1017`、
//! `kimi/kimi-k3.1-pro-highspeed`、`cmaas-glm-5.3`）⇒ ⇒ 前缀匹配会漏掉一大半
//! 真实 id ⇒ ⇒ 模式一律用 `*` 包裹（**子串出现即命中**）。
//!
//! 代价：`*gpt-*` 也会命中 `my-gpt-4` 这类中转站别名 —— **那正是想要的
//! 行为**（它确实是 GPT 的转发）。
//!
//! ⚠️ **没有前导 `*` 的模式是严格前缀**（`o1*` 不许命中 `proto1`）。

/// glob 简化版：`*` 匹配任意串（首段在无前导通配时按**前缀**命中）。
pub(crate) fn matches(pattern: &str, id: &str) -> bool {
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
        // ⚠️ 用 `get` 而不是直接切片 —— id 里可能有非 ASCII（模型名可以是中文）
        // ⇒ ⇒ 字节下标落在字符中间会 panic ⇒ ⇒ 这里判 None 当**不匹配**。
        let Some(rest) = id.get(cursor..) else {
            return false;
        };
        if i == 0 && !leading_wildcard {
            // 无前导 `*` ⇒ 首段必须**前缀**命中（`o1*` 不许命中 `proto1`）。
            if !rest.starts_with(part) {
                return false;
            }
            cursor += part.len();
            continue;
        }
        match rest.find(part) {
            Some(pos) => cursor += pos + part.len(),
            None => return false,
        }
    }
    pattern.ends_with('*') || cursor == id.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_wildcard_means_substring() {
        assert!(matches("*gpt-*", "my-gpt-4"));
        assert!(matches("*gpt-*", "openai/gpt-4o"));
        assert!(!matches("*gpt-*", "claude-sonnet-4"));
    }

    #[test]
    fn no_leading_wildcard_means_strict_prefix() {
        assert!(matches("o1*", "o1-mini"));
        assert!(!matches("o1*", "proto1"));
        assert!(!matches("asr-*", "MiniMax/asr-1.0"));
    }

    #[test]
    fn an_exact_pattern_needs_an_exact_id() {
        assert!(matches("gpt-4o", "gpt-4o"));
        assert!(!matches("gpt-4o", "gpt-4o-mini"));
    }

    /// ⚠️ 非 ASCII 的 id 不许 panic（字节切片会切在字符中间）。
    #[test]
    fn non_ascii_ids_do_not_panic() {
        assert!(matches("*通义*", "通义千问-max"));
        assert!(!matches("*通义*", "别的模型"));
    }
}
