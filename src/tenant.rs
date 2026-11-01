//! **海马体租户 id 的派生** —— 共享层**一处实现**（杏仁核与端脑都走它）。
//!
//! ## 为什么需要它
//!
//! 海马体是**多租户**记忆服务 ⇒ ⇒ 「这条记忆属于谁」由 `tenantId` 决定
//! ⇒ ⇒ 而端脑现在会把「这一轮是谁」（principal）一路递下来
//! ⇒ ⇒ 叶区要把它**变成租户** ⇒ ⇒ **派生规则必须只有一处**
//! （杏仁核一份、端脑一份 ⇒ ⇒ 迟早漂 ⇒ ⇒ 漂了就是「读到别人的记忆」）。
//!
//! ## 租户 id 的硬约束
//!
//! 源：海马体服务端 `hippocampus-server/src/auth.rs::is_valid_tenant`
//!
//! - 非空；
//! - ≤ **64** 字符；
//! - 只允许 `[A-Za-z0-9._-]` ⇒ ⇒ **`:` 不行** ⇒ ⇒ 端脑的
//!   `principal:alice` **不能**原样当租户 ⇒ ⇒ 必须 slug 化。
//!
//! ⚠️ 本模块把那条规则**抄了一份**（共享层不能依赖 hippocampus-server）
//! ⇒ ⇒ 抄错 ⇒ ⇒ 海马体会拒 ⇒ ⇒ 所以这里**自己也先挡一道**：
//! 派生产物**永远**满足上面三条。

/// 海马体允许的最长租户 id（与服务端同一条规则）。
pub const MAX_TENANT_LEN: usize = 64;

/// 海马体服务端那条规则（抄自 `hippocampus-server/src/auth.rs::is_valid_tenant`）。
pub fn is_valid_tenant(tenant: &str) -> bool {
    !tenant.is_empty()
        && tenant.len() <= MAX_TENANT_LEN
        && tenant
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// 把任意字符串压成**租户安全的** slug（只留 `[A-Za-z0-9._-]`，其余变 `-`）。
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        let keep = c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';
        let ch = if keep { c } else { '-' };
        if ch == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
        } else {
            last_dash = false;
        }
        out.push(ch);
    }
    out.trim_matches('-').to_string()
}

/// FNV-1a（64 位）—— 只用来给**超长**的身份留一个短后缀，**不是**安全哈希。
fn fnv1a64(data: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 把 `{base}.{who}` 塞进 64 字符里；超了就截断并留一个哈希后缀。
fn fit(base: &str, who: &str) -> String {
    let candidate = format!("{base}.{who}");
    if candidate.len() <= MAX_TENANT_LEN {
        return candidate;
    }
    let suffix = format!("-{:x}", fnv1a64(who));
    let budget = MAX_TENANT_LEN.saturating_sub(base.len() + 1 + suffix.len());
    if budget == 0 {
        // base 自己就太长 ⇒ ⇒ 连 base 一起截 ⇒ ⇒ 谁那一位只留一个占位。
        let keep_base = MAX_TENANT_LEN.saturating_sub(suffix.len() + 2);
        let b: String = base.chars().take(keep_base).collect();
        return format!("{b}.x{suffix}");
    }
    let head: String = who.chars().take(budget).collect();
    format!("{base}.{head}{suffix}")
}

/// **这个人的记忆住在哪个租户**。
///
/// - `base` —— 部署的基名（杏仁核的 `AMYGDALA_TENANT`，默认 `default`）
///   ⇒ ⇒ 它把不同部署分开；
/// - `principal` —— 谁（端脑递下来的 principal / 账号）⇒ ⇒ 它把不同的人分开；
///   **空 ⇒ 没人指名 ⇒ ⇒ 落回基名**（与「没人就不带这个键」同一条规矩）。
///
/// ⚠️ 派生出来的 id **永远**满足 `is_valid_tenant` ⇒ ⇒ 不会在海马体那边被拒。
pub fn tenant_id(base: &str, principal: &str) -> String {
    let mut base_slug = slugify(base);
    if base_slug.is_empty() {
        base_slug = "default".to_string();
    }
    // ⚠️ **空身份与「slug 后为空的身份」是两回事**
    // ⇒ ⇒ 空 = 没人指名 ⇒ ⇒ 落回基名；
    // ⇒ ⇒ 非空但 slug 没了（全是怪字符）⇒ ⇒ 用哈希占位 ⇒ ⇒ 仍然各自分开。
    if principal.is_empty() {
        return base_slug;
    }
    let who = slugify(principal);
    if who.is_empty() {
        // slug 之后空了（例：身份全是怪字符）⇒ ⇒ 用哈希占位
        // ⇒ ⇒ 不同的人**仍然**分开（而不是都落回基名）。
        let hash = format!("x{:x}", fnv1a64(principal));
        return fit(&base_slug, &hash);
    }
    fit(&base_slug, &who)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derived_tenant_is_always_valid_for_the_hippocampus() {
        let cases = [
            ("default", "alice"),
            ("default", "principal:alice"),
            ("default", "principal:alice@example.com"),
            ("default", "张 伟"),
            ("default", ""),
            ("a-very-long-deployment-base-name-that-is-long", "principal:bob"),
            ("d", &"x".repeat(200)),
        ];
        for (base, who) in cases {
            let t = tenant_id(base, who);
            assert!(
                is_valid_tenant(&t),
                "租户 {t:?} 不合法（base={base:?} who={who:?}）"
            );
        }
    }

    #[test]
    fn the_persona_part_survives_the_slug() {
        assert_eq!(tenant_id("default", "alice"), "default.alice");
        // ⚠️ 端脑的 principal 形如 `principal:alice` ⇒ ⇒ 冒号不在允许集里
        assert_eq!(tenant_id("default", "principal:alice"), "default.principal-alice");
    }

    #[test]
    fn two_people_never_share_a_tenant() {
        assert_ne!(tenant_id("default", "alice"), tenant_id("default", "bob"));
        assert_ne!(
            tenant_id("default", "alice"),
            tenant_id("other-base", "alice"),
            "不同部署也要分开"
        );
    }

    #[test]
    fn nobody_named_falls_back_to_the_base_tenant() {
        assert_eq!(tenant_id("default", ""), "default");
    }

    #[test]
    fn an_identity_that_slugs_to_nothing_still_gets_its_own_tenant() {
        let t = tenant_id("default", "***");
        assert!(is_valid_tenant(&t));
        assert_ne!(t, "default", "不能都落回基名 ⇒ ⇒ 那就又变成共桶");
    }

    #[test]
    fn the_hippocampus_rule_is_mirrored_not_guessed() {
        // 抄错了就会在这里露馅 ⇒ ⇒ 规则与服务端一致才绿。
        assert!(is_valid_tenant("default.alice"));
        assert!(!is_valid_tenant(""));
        assert!(!is_valid_tenant("has space"));
        assert!(!is_valid_tenant("has:colon"));
        assert!(!is_valid_tenant(&"x".repeat(65)));
    }
}
