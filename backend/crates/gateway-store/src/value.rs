//! Store 值对象、持久化校验与 revision 转换

use std::{fmt, num::NonZeroU64};

use gateway_admin::{
    model::Revision as AdminRevision,
    ports::store::{AdminStoreError, AdminStoreErrorKind, AdminStoreResult},
};
use serde_json::{Map, Value};

use crate::{StoreError, StoreResult, admin_store_error};

pub(crate) fn store_revision(revision: AdminRevision) -> AdminStoreResult<Revision> {
    Revision::new(revision.get()).map_err(|error| admin_store_error("config revision", error))
}

pub(crate) fn admin_revision(revision: Revision) -> AdminStoreResult<AdminRevision> {
    AdminRevision::new(revision.get()).map_err(|_| {
        AdminStoreError::new(
            AdminStoreErrorKind::Invalid,
            "config revision",
            "config revision is invalid",
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(NonZeroU64);

impl Revision {
    pub fn new(value: u64) -> StoreResult<Self> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or_else(|| StoreError::InvalidData {
                source: None,
                entity: "revision",
                message: "must be greater than zero".to_owned(),
            })
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

pub use gateway_admin::model::observability::DecimalAmount;

/// Provider-owned JSON object
/// Store 只验证 object 与大小，不解释内部 key
#[derive(Clone, PartialEq)]
pub struct JsonObject(Map<String, Value>);

impl JsonObject {
    pub fn try_from_value(
        entity: &'static str,
        value: Value,
        max_serialized_bytes: usize,
    ) -> StoreResult<Self> {
        let serialized_bytes = serde_json::to_vec(&value)
            .map_err(|error| StoreError::InvalidData {
                source: Some(error.into()),
                entity,
                message: "JSON encoding failed".to_owned(),
            })?
            .len();
        let Value::Object(fields) = value else {
            return Err(StoreError::InvalidData {
                source: None,
                entity,
                message: "top-level JSON value must be an object".to_owned(),
            });
        };
        if serialized_bytes > max_serialized_bytes {
            return Err(StoreError::InvalidData {
                source: None,
                entity,
                message: format!("serialized JSON exceeds {max_serialized_bytes} bytes"),
            });
        }
        Ok(Self(fields))
    }

    #[must_use]
    pub fn as_value(&self) -> Value {
        Value::Object(self.0.clone())
    }

    #[must_use]
    pub fn fields(&self) -> &Map<String, Value> {
        &self.0
    }
}

impl fmt::Debug for JsonObject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JsonObject([REDACTED])")
    }
}

pub(crate) fn require_nonempty(
    entity: &'static str,
    field: &'static str,
    value: &str,
) -> StoreResult<()> {
    if value.trim().is_empty() {
        Err(StoreError::InvalidData {
            source: None,
            entity,
            message: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}
