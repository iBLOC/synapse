//! Rule-based PII detection (Chinese + Western patterns).
//!
//! Deterministic regex packs with cheap validators (Luhn, CN resident-ID
//! checksum, IPv4 octets) to keep false positives down. Person/address
//! detection needs NER and is intentionally out of scope here — the
//! [`PiiDetector`] trait is the seam for a future neuron-backed
//! detector.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// What kind of sensitive value a span holds.
///
/// Variant order is also the report order; `priority()` is the
/// overlap-resolution order (more specific categories win).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiCategory {
    ApiKey,
    IdCardCn,
    Email,
    BankCard,
    Phone,
    IpAddress,
}

impl PiiCategory {
    /// Stable token label used inside pseudonyms (`[PHONE_1]`).
    pub fn label(self) -> &'static str {
        match self {
            PiiCategory::ApiKey => "API_KEY",
            PiiCategory::IdCardCn => "ID_CARD",
            PiiCategory::Email => "EMAIL",
            PiiCategory::BankCard => "BANK_CARD",
            PiiCategory::Phone => "PHONE",
            PiiCategory::IpAddress => "IP_ADDR",
        }
    }

    /// Lower rank wins when two validated candidates overlap.
    fn priority(self) -> u8 {
        match self {
            PiiCategory::ApiKey => 0,
            PiiCategory::IdCardCn => 1,
            PiiCategory::Email => 2,
            PiiCategory::BankCard => 3,
            PiiCategory::Phone => 4,
            PiiCategory::IpAddress => 5,
        }
    }

    pub const ALL: [PiiCategory; 6] = [
        PiiCategory::ApiKey,
        PiiCategory::IdCardCn,
        PiiCategory::Email,
        PiiCategory::BankCard,
        PiiCategory::Phone,
        PiiCategory::IpAddress,
    ];
}

/// A detected sensitive region. Offsets are **byte** offsets into the
/// input text, char-aligned (regex match boundaries always are).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub category: PiiCategory,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

/// Seam for pluggable detectors (regex today, NER via neuron later).
pub trait PiiDetector: Send + Sync {
    /// Detect spans in `text`. Implementations must return
    /// non-overlapping spans sorted by `start`; the engine skips
    /// overlaps defensively anyway.
    fn detect(&self, text: &str) -> Vec<Span>;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Validator {
    /// Luhn check on the digit sequence (bank cards).
    Luhn,
    /// GB11643-1999 checksum (CN resident ID, 18 digits).
    IdCardCn,
    /// Every octet ≤ 255.
    Ipv4,
}

struct Rule {
    category: PiiCategory,
    regex: Regex,
    /// Capture group carrying the sensitive value (0 = whole match,
    /// 1 = generic `password: <value>` style captures).
    group: usize,
    /// Reject when the match touches an adjacent ASCII digit (keeps
    /// phone/ID spans out of longer digit runs).
    digit_boundary: bool,
    validator: Option<Validator>,
}

/// Regex + validator PII detector. Construct once per engine; rules
/// are immutable after build.
pub struct RegexPiiDetector {
    rules: Vec<Rule>,
}

impl RegexPiiDetector {
    /// All built-in categories enabled.
    pub fn new() -> Self {
        Self::with_categories(&PiiCategory::ALL)
    }

    /// Only the listed categories are detected. Use to disable noisy
    /// ones (e.g. `IpAddress` around version-string-heavy text).
    pub fn with_categories(categories: &[PiiCategory]) -> Self {
        Self {
            rules: default_rules()
                .into_iter()
                .filter(|r| categories.contains(&r.category))
                .collect(),
        }
    }
}

impl Default for RegexPiiDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiDetector for RegexPiiDetector {
    fn detect(&self, text: &str) -> Vec<Span> {
        let mut candidates: Vec<Span> = Vec::new();
        for rule in &self.rules {
            for caps in rule.regex.captures_iter(text) {
                let Some(m) = caps.get(rule.group) else { continue };
                let raw = &text[m.start()..m.end()];
                if rule.digit_boundary && !digit_bounded(text, m.start(), m.end()) {
                    continue;
                }
                let valid = match rule.validator {
                    Some(Validator::Luhn) => {
                        let digits: String =
                            raw.chars().filter(|c| *c != ' ' && *c != '-').collect();
                        (13..=19).contains(&digits.len()) && luhn_ok(&digits)
                    }
                    Some(Validator::IdCardCn) => idcard_ok(raw),
                    Some(Validator::Ipv4) => ipv4_ok(raw),
                    None => true,
                };
                if valid {
                    candidates.push(Span {
                        category: rule.category,
                        start: m.start(),
                        end: m.end(),
                    });
                }
            }
        }
        resolve_overlaps(candidates)
    }
}

fn default_rules() -> Vec<Rule> {
    fn rule(
        category: PiiCategory,
        pattern: &str,
        group: usize,
        digit_boundary: bool,
        validator: Option<Validator>,
    ) -> Rule {
        Rule {
            category,
            regex: Regex::new(pattern).expect("built-in PII pattern must compile"),
            group,
            digit_boundary,
            validator,
        }
    }
    vec![
        // CN mobile: 1[3-9] + 9 digits.
        rule(PiiCategory::Phone, r"1[3-9][0-9]{9}", 0, true, None),
        // CN resident ID: 17 digits + check char. Checksum-validated.
        rule(
            PiiCategory::IdCardCn,
            r"[0-9]{17}[0-9Xx]",
            0,
            true,
            Some(Validator::IdCardCn),
        ),
        rule(
            PiiCategory::Email,
            r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}",
            0,
            false,
            None,
        ),
        // Bank card: 13-19 digits with optional space/dash grouping,
        // Luhn-validated. Longer than phone/ID so overlap resolution
        // prefers it inside long digit runs.
        rule(
            PiiCategory::BankCard,
            r"(?:[0-9][ \-]?){12,18}[0-9]",
            0,
            true,
            Some(Validator::Luhn),
        ),
        // Well-known token formats (order matters: sk-ant- before sk-).
        rule(
            PiiCategory::ApiKey,
            r"sk-ant-[A-Za-z0-9_\-]{16,}|sk-[A-Za-z0-9_\-]{20,}|gh[pousr]_[A-Za-z0-9]{16,}|xox[baprs]-[A-Za-z0-9\-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_\-]{30,}",
            0,
            false,
            None,
        ),
        // Generic secret assignment — only the captured value is
        // redacted, the label stays readable for task context.
        rule(
            PiiCategory::ApiKey,
            r#"(?i)(?:api[_\-]?key|access[_\-]?key|secret|token|passwd|password|密钥|密码|口令|私钥)["']?[ \t]*[:=：][ \t]*["']?([A-Za-z0-9_\-!@#$%^&*+=.,:/\\~?]{6,})"#,
            1,
            false,
            None,
        ),
        rule(
            PiiCategory::IpAddress,
            r"[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}",
            0,
            false,
            Some(Validator::Ipv4),
        ),
    ]
}

/// No ASCII digit may touch the match on either side.
fn digit_bounded(text: &str, start: usize, end: usize) -> bool {
    let b = text.as_bytes();
    (start == 0 || !b[start - 1].is_ascii_digit()) && (end == b.len() || !b[end].is_ascii_digit())
}

/// Longest span wins; on equal length the more specific category
/// (lower priority rank) wins; identical/overlapping losers drop.
fn resolve_overlaps(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then(a.category.priority().cmp(&b.category.priority()))
            .then(a.start.cmp(&b.start))
    });
    let mut kept: Vec<Span> = Vec::new();
    for s in spans {
        if kept.iter().all(|k| s.end <= k.start || s.start >= k.end) {
            kept.push(s);
        }
    }
    kept.sort_by_key(|s| s.start);
    kept
}

fn luhn_ok(digits: &str) -> bool {
    let bytes: Vec<u8> = digits.bytes().filter(|b| b.is_ascii_digit()).collect();
    if bytes.len() < 2 {
        return false;
    }
    let mut sum = 0u32;
    for (i, b) in bytes.iter().enumerate() {
        let d = (b - b'0') as u32;
        // Double every second digit from the right (check digit last).
        if (bytes.len() - i) % 2 == 0 {
            let doubled = d * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += d;
        }
    }
    sum % 10 == 0
}

fn idcard_ok(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() != 18 {
        return false;
    }
    let weights = [7u32, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    let mut sum = 0u32;
    for (i, b) in bytes.iter().take(17).enumerate() {
        if !b.is_ascii_digit() {
            return false;
        }
        sum += (b - b'0') as u32 * weights[i];
    }
    let expected = match sum % 11 {
        0 => b'1',
        1 => b'0',
        2 => b'X',
        3 => b'9',
        4 => b'8',
        5 => b'7',
        6 => b'6',
        7 => b'5',
        8 => b'4',
        9 => b'3',
        _ => b'2',
    };
    let actual = bytes[17].to_ascii_uppercase();
    actual == expected
}

fn ipv4_ok(raw: &str) -> bool {
    raw.split('.').all(|o| o.parse::<u8>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats(text: &str) -> Vec<PiiCategory> {
        RegexPiiDetector::new()
            .detect(text)
            .into_iter()
            .map(|s| s.category)
            .collect()
    }

    #[test]
    fn phone_in_chinese_text() {
        assert_eq!(cats("请回拨 13800138000 谢谢"), vec![PiiCategory::Phone]);
    }

    #[test]
    fn phone_inside_longer_digit_run_rejected() {
        assert!(cats("订单号 9138001380002 已发货").is_empty());
    }

    #[test]
    fn valid_idcard_detected() {
        // Classic checksum-valid example (Beijing, 1949-12-31).
        assert_eq!(
            cats("身份证号 11010519491231002X"),
            vec![PiiCategory::IdCardCn]
        );
    }

    #[test]
    fn bad_checksum_idcard_not_classified_as_id() {
        let got = cats("110105194912310021");
        assert!(!got.contains(&PiiCategory::IdCardCn));
    }

    #[test]
    fn valid_idcard_not_double_reported_as_phone() {
        let spans = RegexPiiDetector::new().detect("证件 11010519491231002X 收好");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].category, PiiCategory::IdCardCn);
    }

    #[test]
    fn email_detected() {
        assert_eq!(
            cats("发到 zhang.san@example.com 吧"),
            vec![PiiCategory::Email]
        );
    }

    #[test]
    fn apikey_known_prefix_detected() {
        assert_eq!(cats("key sk-abc123def456ghi789jklmno"), vec![PiiCategory::ApiKey]);
        assert_eq!(cats("ghp_16CharAccessTokenX9"), vec![PiiCategory::ApiKey]);
        // Assembled at runtime so the source never spells a full
        // doc-example key literal.
        let aws_key = format!("AKIA{}", "IOSFODNN7EXAMPLE");
        assert_eq!(cats(&aws_key), vec![PiiCategory::ApiKey]);
    }

    #[test]
    fn apikey_generic_label_captures_value_only() {
        let text = "数据库密码：hunter42 不要外传";
        let spans = RegexPiiDetector::new().detect(text);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].category, PiiCategory::ApiKey);
        assert_eq!(&text[spans[0].start..spans[0].end], "hunter42");
    }

    #[test]
    fn bankcard_luhn_valid_with_separators() {
        let text = "卡号 4111 1111 1111 1111 刷卡";
        let spans = RegexPiiDetector::new().detect(text);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].category, PiiCategory::BankCard);
        assert_eq!(&text[spans[0].start..spans[0].end], "4111 1111 1111 1111");
    }

    #[test]
    fn bankcard_luhn_invalid_rejected() {
        assert!(cats("4111111111111112").is_empty());
    }

    #[test]
    fn ipv4_detected_and_invalid_octet_rejected() {
        assert_eq!(cats("内网 192.168.1.23 走专线"), vec![PiiCategory::IpAddress]);
        assert!(cats("版本 999.1.1.1 不匹配").is_empty());
    }

    #[test]
    fn with_categories_filters_rules() {
        let detector = RegexPiiDetector::with_categories(&[PiiCategory::Phone]);
        let spans = detector.detect("13800138000 和 test@example.com 和 192.168.1.1");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].category, PiiCategory::Phone);
    }

    #[test]
    fn empty_text_yields_nothing() {
        assert!(RegexPiiDetector::new().detect("").is_empty());
    }
}
