//! 按 Key 持久化 USD 限额与已取得费用，不依赖尽力写入的请求观测

use std::time::SystemTime;

use futures::future::BoxFuture;

use crate::{error::GatewayError, metering::Decimal, policy::ClientApiKeyId};

use super::ModelRequestId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientBudgetLimits {
    pub daily_usd: Decimal,
    pub weekly_usd: Decimal,
}

impl ClientBudgetLimits {
    #[must_use]
    pub fn is_limited(self) -> bool {
        self.daily_usd != Decimal::ZERO || self.weekly_usd != Decimal::ZERO
    }

    /// 准入与对外预算查询共用真实账本口径，展示字符串不参与访问判断
    #[must_use]
    pub fn access(self, daily_used: Decimal, weekly_used: Decimal) -> ClientBudgetAccess {
        ClientBudgetAccess {
            daily_limit_reached: self.daily_usd != Decimal::ZERO && daily_used >= self.daily_usd,
            weekly_limit_reached: self.weekly_usd != Decimal::ZERO
                && weekly_used >= self.weekly_usd,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientBudgetAccess {
    pub daily_limit_reached: bool,
    pub weekly_limit_reached: bool,
}

impl ClientBudgetAccess {
    #[must_use]
    pub const fn allowed(self) -> bool {
        !self.daily_limit_reached && !self.weekly_limit_reached
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientBudgetStatus {
    pub limits: ClientBudgetLimits,
    pub daily_used_usd: Decimal,
    pub weekly_used_usd: Decimal,
    pub daily_resets_at: Option<SystemTime>,
    pub weekly_resets_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct ClientBudgetCharge {
    pub key_id: ClientApiKeyId,
    pub request_id: ModelRequestId,
    /// 已取得的 USD 费用，包含重试；缺少费用的尝试按零累计
    pub amount_usd: Decimal,
    pub completed_at: SystemTime,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("client budget store is unavailable")]
pub struct ClientBudgetError(#[source] pub Option<crate::error::ErrorSource>);

pub trait ClientBudgetPort: Send + Sync {
    /// 检查当前限额、已用金额与已接管的费用，不预占未来费用
    fn admit(&self, key_id: ClientApiKeyId) -> BoxFuture<'_, Result<(), GatewayError>>;

    /// 按网关请求 ID 幂等接管已取得费用；运行时入队后返回，由 Store 后台落库和重试
    fn settle(&self, charge: ClientBudgetCharge) -> BoxFuture<'_, Result<(), ClientBudgetError>>;
}
