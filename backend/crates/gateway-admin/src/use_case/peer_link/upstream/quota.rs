//! 消费互联明确的额度访问结论，金额只由展示层解释

use std::time::SystemTime;

use gateway_core::account::{QuotaAccessState, QuotaEvidence, QuotaState};
use gateway_protocol::peer::{PeerBudgetWindow, PeerComponentStatus, PeerKeyUsage, PeerLink};

use super::snapshot::{component_fresh, fresh, timestamp};

/// 原始时间与请求开始时间共同限制观测，迟到响应不能覆盖请求期间的新失败
pub(super) fn observe(
    peer: &PeerLink,
    started_at: SystemTime,
    now: SystemTime,
) -> Option<QuotaState> {
    if !fresh(&peer.observed_at, now)
        || !component_fresh(Some(&peer.observed_at), Some(&peer.expires_at), now)
        || peer.usage.status != PeerComponentStatus::Available
        || !component_fresh(peer.usage.observed_at.as_deref(), None, now)
    {
        return None;
    }
    let observed_at = started_at
        .min(timestamp(&peer.observed_at)?)
        .min(timestamp(peer.usage.observed_at.as_deref()?)?);
    let usage = peer.usage.data.as_ref()?;
    match access(usage, now) {
        QuotaAccessState::Allowed => Some(QuotaState::allowed(observed_at)),
        QuotaAccessState::Exhausted => {
            let reset_at = [&usage.daily, &usage.weekly]
                .into_iter()
                .filter(|window| window_access(window, now) == QuotaAccessState::Exhausted)
                .filter_map(|window| window.resets_at.as_deref().and_then(timestamp))
                .min();
            Some(QuotaState::exhausted(
                if usage.allowed == Some(false) {
                    QuotaEvidence::ProviderDenied
                } else {
                    QuotaEvidence::AccountLimitReached
                },
                observed_at,
                reset_at,
            ))
        }
        QuotaAccessState::Unknown => None,
    }
}

/// 与 OAuth 相同，明确的 allowed 优先，窗口耗尽仅补充缺失的访问结论
pub(super) fn access(usage: &PeerKeyUsage, now: SystemTime) -> QuotaAccessState {
    match usage.allowed {
        Some(true) => QuotaAccessState::Allowed,
        Some(false) => QuotaAccessState::Exhausted,
        None if [&usage.daily, &usage.weekly]
            .into_iter()
            .any(|window| window_access(window, now) == QuotaAccessState::Exhausted) =>
        {
            QuotaAccessState::Exhausted
        }
        None => QuotaAccessState::Unknown,
    }
}

/// 过期窗口保持未知，不从重置时间或金额推断恢复
pub(super) fn window_access(window: &PeerBudgetWindow, now: SystemTime) -> QuotaAccessState {
    if window
        .resets_at
        .as_deref()
        .and_then(timestamp)
        .is_some_and(|reset| reset <= now)
    {
        return QuotaAccessState::Unknown;
    }
    match window.limit_reached {
        Some(true) => QuotaAccessState::Exhausted,
        Some(false) => QuotaAccessState::Allowed,
        None => QuotaAccessState::Unknown,
    }
}
