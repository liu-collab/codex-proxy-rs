//! 客户端失败轮次的短期重试抑制与终态前状态更新

use std::{sync::Arc, time::Duration};

use futures::{FutureExt, future::BoxFuture, pin_mut, select_biased};
use futures_timer::Delay;

use super::ModelRequestId;
use crate::{error::StoreError, operation::ClientTurn, policy::ClientApiKeyId};

const FAILURE_TTL: Duration = Duration::from_secs(300);
const STORE_TIMEOUT: Duration = Duration::from_millis(100);

/// 不同客户端不能共享轮次失败状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRetryKey {
    pub client_key_id: ClientApiKeyId,
    pub turn: ClientTurn,
}

/// 可丢失的失败标记，版本使用创建该标记的模型请求 ID
pub trait ClientRetryStore: Send + Sync {
    fn load<'a>(
        &'a self,
        key: &'a ClientRetryKey,
    ) -> BoxFuture<'a, Result<Option<ModelRequestId>, StoreError>>;

    fn record_failure<'a>(
        &'a self,
        key: &'a ClientRetryKey,
        version: &'a ModelRequestId,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// 比较与删除必须原子执行，迟到成功不能删除较新的失败
    fn clear_failure<'a>(
        &'a self,
        key: &'a ClientRetryKey,
        expected: &'a ModelRequestId,
    ) -> BoxFuture<'a, Result<(), StoreError>>;
}

pub(super) struct ClientRetryGuard {
    store: Arc<dyn ClientRetryStore>,
    key: ClientRetryKey,
    request_id: ModelRequestId,
    observed: Option<ModelRequestId>,
    settled: bool,
}

impl ClientRetryGuard {
    pub(super) async fn load(
        store: Arc<dyn ClientRetryStore>,
        key: ClientRetryKey,
        request_id: ModelRequestId,
    ) -> Self {
        let observed = bounded("load", &request_id, store.load(&key))
            .await
            .flatten();
        Self {
            store,
            key,
            request_id,
            observed,
            settled: false,
        }
    }

    pub(super) fn has_failure(&self) -> bool {
        self.observed.is_some()
    }

    pub(super) async fn failure(&mut self) {
        if self.settled {
            return;
        }
        bounded(
            "record",
            &self.request_id,
            self.store
                .record_failure(&self.key, &self.request_id, FAILURE_TTL),
        )
        .await;
        self.settled = true;
    }

    pub(super) async fn success(&mut self) {
        if self.settled {
            return;
        }
        if let Some(observed) = &self.observed {
            bounded(
                "clear",
                &self.request_id,
                self.store.clear_failure(&self.key, observed),
            )
            .await;
        }
        self.settled = true;
    }
}

async fn bounded<T>(
    action: &'static str,
    request_id: &ModelRequestId,
    operation: BoxFuture<'_, Result<T, StoreError>>,
) -> Option<T> {
    let operation = operation.fuse();
    let timeout = Delay::new(STORE_TIMEOUT).fuse();
    pin_mut!(operation, timeout);
    select_biased! {
        result = operation => match result {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(action, request_id = request_id.as_str(), error_kind = ?error.kind(), "客户端重试标记不可用，保留原请求结果与重试策略");
                None
            }
        },
        () = timeout => {
            tracing::warn!(action, request_id = request_id.as_str(), "客户端重试标记操作超时，保留原请求结果与重试策略");
            None
        },
    }
}
