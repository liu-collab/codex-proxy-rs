//! 插件包校验、检查、兼容性与解压资源的公共入口

mod cache;
mod compatibility;
mod icon;
mod inspection;
mod state_schema;
mod validation;

pub use cache::PreparedPackage;
pub(crate) use compatibility::{
    host_fingerprint as compatibility_host_fingerprint, requirements as compatibility_requirements,
    warning as compatibility_warning,
};
pub use inspection::PackageInspector;
pub(crate) use inspection::inspection_error;
pub(crate) use state_schema::{
    configuration as state_configuration, validators as state_validators,
};
pub use validation::{PackageError, PackageLimits, ValidatedPackage};
