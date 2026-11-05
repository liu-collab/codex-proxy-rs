//! 账号累计消耗账本的只读入口

use gateway_admin::model::observability::DecimalAmount;
use std::str::FromStr;

use super::*;

/// 读取单个账号在各币种下的累计消耗
///
/// 账本按主键精确匹配账号，缺失即视为尚无消耗，不产生错误也不回退到用量聚合
pub(super) async fn load_cost_total(
    pool: &PgPool,
    account_id: &str,
) -> AdminStoreResult<Vec<AccountCost>> {
    validate_admin_account_ids(std::slice::from_ref(&account_id.to_owned()))
        .map_err(|error| admin_store_error(ENTITY, error))?;
    let rows = sqlx::query_as::<_, (String, String)>(
        "select cost_currency, cost_amount::text
           from provider_account_cost_totals
          where provider_account_ref = $1
          order by cost_currency",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await
    .map_err(|source| {
        admin_store_error(
            ENTITY,
            postgres_unavailable("load account cost total", source),
        )
    })?;
    rows.into_iter()
        .map(|(currency, amount)| {
            // numeric(20,10) 先转文本再解析，避免在 SQL 与 Rust 之间引入浮点舍入
            let amount = DecimalAmount::from_str(&amount).map_err(|error| {
                admin_store_error(
                    ENTITY,
                    StoreError::InvalidData {
                        source: Some(error.into()),
                        entity: "account cost total",
                        message: "expected a non-negative numeric(20,10) value".to_owned(),
                    },
                )
            })?;
            Ok(AccountCost { currency, amount })
        })
        .collect()
}
