//! 系统更新各基础设施步骤共用的管理端错误分类

use gateway_admin::ports::system::{SystemOperationError, SystemOperationErrorKind};

pub(super) type OperationError = SystemOperationError;

pub(super) fn invalid(message: impl Into<String>) -> OperationError {
    SystemOperationError::new(SystemOperationErrorKind::Invalid, message)
}

pub(super) fn conflict(message: impl Into<String>) -> OperationError {
    SystemOperationError::new(SystemOperationErrorKind::Conflict, message)
}

pub(super) fn upstream(message: impl Into<String>) -> OperationError {
    SystemOperationError::new(SystemOperationErrorKind::Upstream, message)
}

pub(super) fn http(message: &'static str, source: reqwest::Error) -> OperationError {
    let failure = upstream(message);
    let failure = if source.url().is_some() {
        failure.redact_sensitive_context()
    } else {
        failure
    };
    failure.with_source(source.without_url())
}

pub(super) fn internal(message: impl Into<String>) -> OperationError {
    SystemOperationError::new(SystemOperationErrorKind::Internal, message)
}
