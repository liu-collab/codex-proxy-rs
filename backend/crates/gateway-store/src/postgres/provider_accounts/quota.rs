//! 额度快照的轻量版本读取与连接校验，不解释额度正文

use super::*;
use gateway_core::account::{QuotaObservationVersion, UpstreamConnection};

pub(super) async fn versions(
    repository: &PgProviderAccountRepository,
    accounts: &[CoreProviderAccountId],
) -> Result<Vec<QuotaObservationVersion>, CoreStoreError> {
    let ids = accounts
        .iter()
        .map(CoreProviderAccountId::as_str)
        .collect::<Vec<_>>();
    let rows = sqlx::query("select id, credential_revision, quota_observed_at from provider_accounts where id = any($1) and quota_observed_at is not null")
        .bind(ids).fetch_all(&repository.pool).await.map_err(database_error)?;
    rows.into_iter()
        .map(|row| {
            Ok(QuotaObservationVersion {
                account_id: CoreProviderAccountId::new(
                    row.try_get::<String, _>("id").map_err(database_error)?,
                )
                .map_err(|_| CoreStoreError::new(CoreStoreErrorKind::InvalidData))?,
                credential_revision: CoreCredentialRevision::new(
                    u64::try_from(
                        row.try_get::<i64, _>("credential_revision")
                            .map_err(database_error)?,
                    )
                    .map_err(|_| CoreStoreError::new(CoreStoreErrorKind::InvalidData))?,
                )
                .map_err(|_| CoreStoreError::new(CoreStoreErrorKind::InvalidData))?,
                observed_at: row
                    .try_get::<DateTime<Utc>, _>("quota_observed_at")
                    .map_err(database_error)?
                    .into(),
            })
        })
        .collect()
}

pub(super) async fn write(
    repository: &PgProviderAccountRepository,
    observation: QuotaObservation,
) -> Result<QuotaWriteOutcome, CoreStoreError> {
    let quota = JsonObject::try_from_value(
        "provider_quota_json",
        serde_json::Value::Object(observation.quota.into_inner()),
        QUOTA_MAX_BYTES,
    )
    .map_err(crate::core_store_error)?;
    let revision =
        Revision::new(observation.expected_revision.get()).map_err(crate::core_store_error)?;
    let updated = if let Some(expected) = observation.expected_connection {
        if expected.account_id != observation.account_id
            || expected.credential_revision != observation.expected_revision
        {
            return Ok(QuotaWriteOutcome::Conflict);
        }
        let mut transaction = repository.pool.begin().await.map_err(database_error)?;
        // 先锁账号，再用新语句读取代理投影，避免等待行锁期间提交的变更使用旧语句快照
        sqlx::query("select id from provider_accounts where id = $1 for update")
            .bind(observation.account_id.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(database_error)?;
        let row = sqlx::query(ACCOUNT_SELECT)
            .bind(observation.account_id.as_str())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(database_error)?;
        let Some(row) = row else {
            return Ok(QuotaWriteOutcome::Conflict);
        };
        let account = core_account_from_summary(
            account_record_from_row(row)
                .map_err(crate::core_store_error)?
                .summary,
        )?;
        if !account.enabled() || UpstreamConnection::for_account(&account) != expected {
            return Ok(QuotaWriteOutcome::Conflict);
        }
        let updated = super::repository::write_quota(
            &mut *transaction,
            observation.account_id.as_str(),
            revision,
            quota,
            observation.observed_at.into(),
            observation.state,
            observation.plan_type.as_deref(),
        )
        .await
        .map_err(crate::core_store_error)?;
        transaction.commit().await.map_err(database_error)?;
        updated
    } else {
        repository
            .compare_and_swap_provider_quota(
                observation.account_id.as_str(),
                revision,
                quota,
                observation.observed_at.into(),
                observation.state,
                observation.plan_type.as_deref(),
            )
            .await
            .map_err(crate::core_store_error)?
    };
    Ok(if updated {
        QuotaWriteOutcome::Updated
    } else {
        QuotaWriteOutcome::Conflict
    })
}

fn database_error(error: sqlx::Error) -> CoreStoreError {
    CoreStoreError::caused_by(CoreStoreErrorKind::Unavailable, error)
}
