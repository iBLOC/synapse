//! Reversible pseudonym mapping: raw value ↔ stable token.
//!
//! In-memory only, per-engine (per provider) lifetime. Never persist
//! this table — it inverts the whole redaction.

use super::detector::PiiCategory;
use parking_lot::Mutex;
use std::collections::HashMap;

#[derive(Default)]
struct Inner {
    next_id: HashMap<PiiCategory, usize>,
    forward: HashMap<String, String>,
    reverse: HashMap<String, String>,
}

/// Bidirectional raw-value ↔ pseudonym map. Tokens look like
/// `[PHONE_1]`; the bracketed form makes tokens non-prefix-colliding
/// (`[PHONE_1]` is never a prefix of `[PHONE_10]`), so plain
/// sequential replacement is safe in any order.
pub struct PseudonymMapper {
    inner: Mutex<Inner>,
}

impl PseudonymMapper {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    /// Get the token for `raw`, assigning the next per-category id on
    /// first sight. The same raw value always maps to the same token
    /// for the lifetime of this mapper.
    pub fn pseudonym_for(&self, category: PiiCategory, raw: &str) -> String {
        let mut inner = self.inner.lock();
        if let Some(token) = inner.forward.get(raw) {
            return token.clone();
        }
        let id = {
            let slot = inner.next_id.entry(category).or_insert(0);
            *slot += 1;
            *slot
        };
        let token = format!("[{}_{}]", category.label(), id);
        inner.forward.insert(raw.to_string(), token.clone());
        inner.reverse.insert(token.clone(), raw.to_string());
        token
    }

    /// Replace every known token in `text` with its original value.
    /// Unknown tokens pass through untouched.
    pub fn restore(&self, text: &str) -> String {
        let inner = self.inner.lock();
        if inner.reverse.is_empty() {
            return text.to_string();
        }
        let mut out = text.to_string();
        for (token, raw) in inner.reverse.iter() {
            if out.contains(token.as_str()) {
                out = out.replace(token.as_str(), raw);
            }
        }
        out
    }

    /// Number of distinct mapped values.
    pub fn len(&self) -> usize {
        self.inner.lock().forward.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PseudonymMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_raw_same_token_across_categories_independent() {
        let mapper = PseudonymMapper::new();
        let a = mapper.pseudonym_for(PiiCategory::Phone, "13800138000");
        let b = mapper.pseudonym_for(PiiCategory::Phone, "13800138000");
        assert_eq!(a, b);
        assert_eq!(a, "[PHONE_1]");
        // The forward map is keyed by raw alone: one raw value always
        // restores from a single token, whichever category found it.
        let c = mapper.pseudonym_for(PiiCategory::Email, "13800138000");
        assert_eq!(c, "[PHONE_1]");
        assert_eq!(mapper.len(), 1);
    }

    #[test]
    fn restore_round_trips_mixed_text() {
        let mapper = PseudonymMapper::new();
        let t1 = mapper.pseudonym_for(PiiCategory::Phone, "13800138000");
        let t2 = mapper.pseudonym_for(PiiCategory::Email, "a@b.co");
        let text = format!("先 {} 后 {} 再 {}", t1, t2, t1);
        assert_eq!(mapper.restore(&text), "先 13800138000 后 a@b.co 再 13800138000");
    }

    #[test]
    fn tokens_do_not_prefix_collide() {
        let mapper = PseudonymMapper::new();
        for i in 0..10 {
            mapper.pseudonym_for(PiiCategory::Phone, &format!("1380013800{i}"));
        }
        let ten = mapper.pseudonym_for(PiiCategory::Phone, "最后一条");
        assert_eq!(ten, "[PHONE_11]");
        let text = format!("{} 和 {}", ten, "[PHONE_1]");
        assert_eq!(mapper.restore(&text), "最后一条 和 13800138000");
    }

    #[test]
    fn restore_untouched_without_mappings() {
        let mapper = PseudonymMapper::new();
        assert_eq!(mapper.restore("plain text"), "plain text");
        assert!(mapper.is_empty());
    }
}
