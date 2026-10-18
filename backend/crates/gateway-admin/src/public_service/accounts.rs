//! 账号自动重置的公开操作声明，复用账号用例及宿主签发的审计上下文

use crate::{AccountsService, model::AdminError};
use gateway_core::account::ProviderAccountId;
use std::sync::Arc;

pub(super) fn register(
    registry: &mut super::Registry,
    service: &Arc<dyn AccountsService>,
) -> Result<(), AdminError> {
    macro_rules! register {
        ($sdk:ident, $method:ident, $input:ty, $pattern:pat $(, $argument:expr)* $(,)?) => {{
            let service = service.clone();
            registry.register(concat!("accounts.", stringify!($method)), move |$pattern: $input| {
                let service = service.clone();
                Box::pin(async move { service.$method($($argument),*).await })
            })?;
        }};
    }
    register!(
        GetResetCreditAutomation,
        reset_credit_automation_status,
        String,
        account_id,
        ProviderAccountId::new(account_id).map_err(|_| AdminError::invalid("账号 ID 无效"))?
    );
    register!(
        SetResetCreditAutomation,
        set_reset_credit_automation,
        (String, bool),
        (account_id, enabled),
        &super::actor::mutation_context()?,
        ProviderAccountId::new(account_id).map_err(|_| AdminError::invalid("账号 ID 无效"))?,
        enabled
    );
    Ok(())
}
