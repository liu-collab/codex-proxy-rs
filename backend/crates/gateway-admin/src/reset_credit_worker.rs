//! 自动重置卡 Worker 的到期扫描、并发预算与取消，消费编排由账号用例负责

use crate::{
    model::AdminError,
    ports::store::{AccountStore, SettingsStore},
    use_case::accounts::AccountsService,
};
use chrono::Utc;
use futures::StreamExt as _;
use gateway_core::{
    account::ProviderAccountId,
    task::{
        ScheduledTask, WorkerContribution, WorkerCycleContext, WorkerId, WorkerKind,
        WorkerLeaseRequest, WorkerRegistration, WorkerRunnable, WorkerSchedule, WorkerTaskError,
    },
};
use std::{sync::Arc, time::Duration};

pub(crate) struct ResetCreditAutomationTask {
    pub(crate) accounts: Arc<dyn AccountsService>,
    pub(crate) store: Arc<dyn AccountStore>,
    pub(crate) settings: Arc<dyn SettingsStore>,
}

impl ResetCreditAutomationTask {
    pub(crate) fn into_worker(self) -> Result<WorkerContribution, AdminError> {
        let id = WorkerId::try_new(WorkerKind::ResetCreditAutomation, "admin")
            .map_err(|_| AdminError::internal("重置卡 Worker ID 不合法"))?;
        let schedule = WorkerSchedule::try_new(
            Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(60),
            Duration::from_secs(90),
            Duration::from_secs(30),
        )
        .map_err(|_| AdminError::internal("重置卡 Worker 调度配置不合法"))?;
        let lease = WorkerLeaseRequest::try_new(id.clone(), Duration::from_secs(90))
            .map_err(|_| AdminError::internal("重置卡 Worker 租约配置不合法"))?;
        let registration = WorkerRegistration::try_new(
            id,
            WorkerRunnable::Scheduled {
                schedule,
                lease: Some(lease),
                task: Box::new(self),
            },
        )
        .map_err(|_| AdminError::internal("重置卡 Worker 注册失败"))?;
        Ok(WorkerContribution::Registration(registration))
    }
}

impl ScheduledTask for ResetCreditAutomationTask {
    fn run_cycle(
        &self,
        context: WorkerCycleContext,
    ) -> futures::future::BoxFuture<'_, Result<(), WorkerTaskError>> {
        Box::pin(async move {
            let settings = self
                .settings
                .load_runtime_settings()
                .await
                .map_err(|_| WorkerTaskError::safe("重置卡设置读取失败"))?;
            // 单轮有界，积压沿到期顺序继续；并发复用已有账号刷新配置
            let due = self
                .store
                .due_reset_credit_automations(Utc::now(), 100)
                .await
                .map_err(|_| WorkerTaskError::safe("重置卡计划读取失败"))?;
            let work = futures::stream::iter(due)
                .map(|account_id| async move {
                    let id = ProviderAccountId::new(account_id.clone())
                        .map_err(|_| WorkerTaskError::safe("重置卡账号 ID 不合法"))?;
                    if let Err(error) = self.accounts.reconcile_reset_credit_automation(id).await {
                        tracing::warn!(
                            account_id,
                            error_kind = error.kind().as_str(),
                            "自动重置卡任务失败"
                        );
                    }
                    Ok::<_, WorkerTaskError>(())
                })
                .buffer_unordered(settings.values.refresh_concurrency.max(1) as usize)
                .collect::<Vec<_>>();
            tokio::select! {
                () = context.cancellation().cancelled() => {},
                _ = tokio::time::timeout(Duration::from_secs(30), work) => {},
            }
            Ok(())
        })
    }
}
