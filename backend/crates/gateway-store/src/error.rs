//! Store 的基础设施错误分类、来源保留与领域端口映射

use gateway_admin::ports::store::{AdminStoreError, AdminStoreErrorKind};

/// 发生错误的基础设施边界
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreBackend {
    PostgreSql,
    Redis,
}

/// 上层状态机需要区分的稳定冲突类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    StaleRevision,
    DuplicateName,
    AlreadyFinalized,
    RequestNotRunning,
    InvalidTransition,
    LeaseLost,
    FencingTokenStale,
}

/// Store adapter 的稳定错误边界
#[derive(Debug, Clone, thiserror::Error)]
pub enum StoreError {
    #[error("{backend:?} store is unavailable: {message}")]
    Unavailable {
        backend: StoreBackend,
        message: String,
        source: Option<gateway_core::error::ErrorSource>,
    },
    #[error("{entity} {id} was not found")]
    NotFound {
        entity: &'static str,
        id: String,
        source: Option<gateway_core::error::ErrorSource>,
    },
    #[error("store conflict for {entity} {id}: {kind:?}")]
    Conflict {
        entity: &'static str,
        id: String,
        kind: ConflictKind,
        source: Option<gateway_core::error::ErrorSource>,
    },
    #[error("invalid persisted {entity}: {message}")]
    InvalidData {
        entity: &'static str,
        message: String,
        source: Option<gateway_core::error::ErrorSource>,
    },
}

impl StoreError {
    /// 本地校验可没有来源；转换已有失败时附加真实原因
    #[must_use]
    pub(crate) fn with_source(
        mut self,
        error: impl Into<gateway_core::error::ErrorSource>,
    ) -> Self {
        let error = error.into();
        match &mut self {
            Self::Unavailable { source, .. }
            | Self::InvalidData { source, .. }
            | Self::Conflict { source, .. }
            | Self::NotFound { source, .. } => *source = Some(error),
        }
        self
    }

    pub(crate) fn with_cleanup(
        mut self,
        cleanup: impl Into<gateway_core::error::ErrorSource>,
    ) -> Self {
        let source = match &mut self {
            Self::Unavailable { source, .. }
            | Self::InvalidData { source, .. }
            | Self::Conflict { source, .. }
            | Self::NotFound { source, .. } => source,
        };
        *source = Some(gateway_core::error::ErrorSource::cleanup(
            source.take(),
            cleanup,
        ));
        self
    }

    fn admin_kind(&self) -> AdminStoreErrorKind {
        match self {
            Self::NotFound { .. } => AdminStoreErrorKind::NotFound,
            Self::Conflict {
                kind: ConflictKind::StaleRevision,
                ..
            } => AdminStoreErrorKind::StaleRevision,
            Self::Conflict {
                kind: ConflictKind::DuplicateName,
                ..
            } => AdminStoreErrorKind::DuplicateName,
            Self::Conflict { .. } => AdminStoreErrorKind::Conflict,
            Self::InvalidData { .. } => AdminStoreErrorKind::Invalid,
            Self::Unavailable { .. } => AdminStoreErrorKind::Unavailable,
        }
    }

    fn core_kind(&self) -> gateway_core::error::StoreErrorKind {
        use gateway_core::error::StoreErrorKind;
        match self {
            Self::Unavailable { .. } => StoreErrorKind::Unavailable,
            Self::Conflict { .. } => StoreErrorKind::Conflict,
            Self::NotFound { .. } | Self::InvalidData { .. } => StoreErrorKind::InvalidData,
        }
    }
}

pub type StoreResult<T> = Result<T, StoreError>;
pub(crate) fn admin_store_error(resource: &'static str, error: StoreError) -> AdminStoreError {
    AdminStoreError::new(error.admin_kind(), resource, "store operation failed").with_source(error)
}

pub(crate) fn postgres_unavailable(
    operation: &'static str,
    source: impl std::error::Error + Send + Sync + 'static,
) -> StoreError {
    StoreError::Unavailable {
        backend: StoreBackend::PostgreSql,
        message: operation.to_owned(),
        source: Some(gateway_core::error::ErrorSource::new(source)),
    }
}

pub(crate) fn redis_unavailable(
    operation: &'static str,
    source: impl std::error::Error + Send + Sync + 'static,
) -> StoreError {
    StoreError::Unavailable {
        backend: StoreBackend::Redis,
        message: operation.to_owned(),
        source: Some(gateway_core::error::ErrorSource::new(source)),
    }
}

/// Provider 存储端口共用不可用分类，实际后端及原因由来源链保留
pub(crate) fn provider_unavailable(
    operation: &'static str,
    source: impl std::error::Error + Send + Sync + 'static,
) -> gateway_core::provider_ports::ProviderStoreError {
    gateway_core::provider_ports::ProviderStoreError::caused_by(
        gateway_core::provider_ports::ProviderStoreErrorKind::Unavailable,
        operation,
        source,
    )
}

pub(crate) fn core_store_error(error: StoreError) -> gateway_core::error::StoreError {
    gateway_core::error::StoreError::caused_by(error.core_kind(), error)
}
