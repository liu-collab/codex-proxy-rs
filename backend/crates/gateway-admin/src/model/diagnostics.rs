//! 故障诊断导出的目标、已采集证据与缺口

use chrono::{DateTime, Utc};

use super::{
    observability::{OpsError, UsageDetail},
    system::SystemVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticTarget {
    Request(String),
    Operation(String),
}

impl DiagnosticTarget {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Request(id) | Self::Operation(id) => id,
        }
    }
}

pub struct DiagnosticGap {
    pub section: String,
    pub reason: &'static str,
}

pub struct DiagnosticRequest {
    pub request_id: String,
    pub detail: Option<UsageDetail>,
    pub errors: Vec<OpsError>,
}

/// 原始受控详情只能经导出投影脱敏后交付，不参与普通日志或 Debug
pub struct DiagnosticBundle {
    pub target: DiagnosticTarget,
    pub collected_at: DateTime<Utc>,
    pub environment: Option<SystemVersion>,
    pub error: Option<OpsError>,
    pub requests: Vec<DiagnosticRequest>,
    pub gaps: Vec<DiagnosticGap>,
}
