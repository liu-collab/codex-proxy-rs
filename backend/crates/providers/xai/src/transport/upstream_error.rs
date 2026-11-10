//! xAI 上游错误的额度分类与客户端可见账号标识脱敏

const UUID_TEXT_LEN: usize = 36;
const FREE_USAGE_SIGNALS: &[&str] = &[
    "subscription:free-usage-exhausted",
    "subscription_free_usage_exhausted",
    "free-usage-exhausted",
    "free_usage_exhausted",
    "usage-limit-exceeded",
    "usage_limit_exceeded",
    "free-tier-limit",
    "free_tier_limit",
    "used all the included free usage",
    "used all your free usage",
    "free quota",
    "no remaining free",
    "out of free",
    "usage resets over a rolling",
    "额度耗尽",
    "额度用完",
    "额度不足",
    "额度已用尽",
    "免费额度",
    "免费用量",
    "用量耗尽",
    "配额耗尽",
    "配额不足",
    "模型额度",
];
const ACCOUNT_QUOTA_SIGNALS: &[&str] = &[
    "personal-team-blocked:spending-limit",
    "personal_team_blocked_spending_limit",
    "quota_exceeded",
    "insufficient_quota",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrokQuotaFailureKind {
    Account,
    FreeAccount,
    FreeModelUsage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokQuotaSignal {
    Account,
    FreeUsage,
}

pub(crate) fn classify_grok_quota_failure(
    code: Option<&str>,
    error_type: Option<&str>,
    message: Option<&str>,
) -> Option<GrokQuotaFailureKind> {
    let code = code.map(str::to_ascii_lowercase);
    let error_type = error_type.map(str::to_ascii_lowercase);
    let message = message.map(str::to_ascii_lowercase);
    // `code` 与 `type` 是稳定的上游字段，冲突的可读消息不能覆盖它们
    // 消息仅用于回退判断，或细化已确认免费额度信号的滚动窗口
    let structured_signal = code
        .as_deref()
        .and_then(classify_quota_signal)
        .or_else(|| error_type.as_deref().and_then(classify_quota_signal));
    match structured_signal {
        Some(GrokQuotaSignal::Account) => Some(GrokQuotaFailureKind::Account),
        Some(GrokQuotaSignal::FreeUsage) => Some(free_usage_failure_kind(message.as_deref())),
        None => classify_message_quota_failure(message.as_deref()),
    }
}

fn classify_quota_signal(value: &str) -> Option<GrokQuotaSignal> {
    if contains_any(value, ACCOUNT_QUOTA_SIGNALS) {
        Some(GrokQuotaSignal::Account)
    } else if contains_any(value, FREE_USAGE_SIGNALS) {
        Some(GrokQuotaSignal::FreeUsage)
    } else {
        None
    }
}

fn classify_message_quota_failure(message: Option<&str>) -> Option<GrokQuotaFailureKind> {
    match message.and_then(classify_quota_signal) {
        Some(GrokQuotaSignal::Account) => Some(GrokQuotaFailureKind::Account),
        Some(GrokQuotaSignal::FreeUsage) => Some(free_usage_failure_kind(message)),
        None => None,
    }
}

fn free_usage_failure_kind(message: Option<&str>) -> GrokQuotaFailureKind {
    if message.is_some_and(|value| value.contains("for model")) {
        GrokQuotaFailureKind::FreeModelUsage
    } else {
        GrokQuotaFailureKind::FreeAccount
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

/// 把 message 中的 UUID 替换为占位符，控制字符归一为空格，
/// 使文本满足客户端可见错误的约束且不携带可定位账号的标识
pub(super) fn scrub_account_fingerprints(message: &str) -> String {
    let bytes = message.as_bytes();
    let mut scrubbed = String::with_capacity(message.len());
    let mut index = 0;
    while index < message.len() {
        if uuid_at(bytes, index) {
            scrubbed.push_str("[redacted]");
            index += UUID_TEXT_LEN;
            continue;
        }
        let character = message[index..].chars().next().expect("char boundary");
        scrubbed.push(if character.is_control() {
            ' '
        } else {
            character
        });
        index += character.len_utf8();
    }
    scrubbed
}

fn uuid_at(bytes: &[u8], index: usize) -> bool {
    let Some(candidate) = bytes.get(index..index + UUID_TEXT_LEN) else {
        return false;
    };
    if index > 0 && bytes[index - 1].is_ascii_alphanumeric() {
        return false;
    }
    if bytes
        .get(index + UUID_TEXT_LEN)
        .is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    candidate.iter().enumerate().all(|(offset, byte)| {
        if matches!(offset, 8 | 13 | 18 | 23) {
            *byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}
