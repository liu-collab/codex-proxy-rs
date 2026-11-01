//! 更新 owner 消费失败时记录受控详情，初始化和关闭阶段保持尽力写入

use std::sync::{Arc, OnceLock};

use gateway_core::diagnostics::{OperationalDiagnostics, OperationalFailure};

use super::OperationError;

#[derive(Clone, Default)]
pub(super) struct Diagnostics(Arc<OnceLock<Arc<dyn OperationalDiagnostics>>>);

impl Diagnostics {
    pub(super) fn bind(&self, diagnostics: Arc<dyn OperationalDiagnostics>) {
        let _ = self.0.set(diagnostics);
    }

    pub(super) async fn record(
        &self,
        operation: &'static str,
        correlation_id: Option<&str>,
        error: &OperationError,
    ) {
        let Some(diagnostics) = self.0.get() else {
            return;
        };
        let mut failure = OperationalFailure::new(
            "system_update",
            operation,
            "system_operation_failed",
            error.message(),
        );
        failure.correlation_id = correlation_id.map(str::to_owned);
        failure.details = error.error_details();
        if diagnostics.record_failure(failure).await.is_err() {
            tracing::warn!(operation, "系统操作诊断记录失败");
        }
    }

    pub(super) fn record_on_drop(
        &self,
        operation: &'static str,
        correlation_id: Option<String>,
        error: OperationError,
    ) {
        // 析构不能等待异步存储，运行时已关闭时沿用诊断尽力写入的合同
        if self.0.get().is_some()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            let diagnostics = self.clone();
            drop(runtime.spawn(async move {
                diagnostics
                    .record(operation, correlation_id.as_deref(), &error)
                    .await;
            }));
        }
    }
}
