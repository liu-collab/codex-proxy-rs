//! 将清单私有状态声明编译为配置、稳定摘要与离线校验器

use std::collections::BTreeMap;

use gateway_admin::model::{
    AdminError,
    plugins::state::{PluginStateConfiguration, PluginStateSchema},
};
use gateway_plugin_sdk::Manifest;
use sha2::{Digest as _, Sha256};

pub(crate) fn configuration(manifest: &Manifest) -> Result<PluginStateConfiguration, AdminError> {
    let _ = validators(manifest)?;
    let mut namespaces = Vec::with_capacity(manifest.state.len());
    for state in &manifest.state {
        let mut schema = state.schema.clone();
        schema.sort_all_objects();
        let encoded = serde_json::to_vec(&schema)
            .map_err(|_| AdminError::invalid("插件状态 schema 无法编码"))?;
        namespaces.push(PluginStateSchema {
            namespace: state.namespace.clone(),
            schema_version: state.schema_version,
            schema_sha256: hex::encode(Sha256::digest(encoded)),
            schema: state.schema.clone(),
            maximum_records: state.maximum_records,
            maximum_bytes: state.maximum_bytes,
            maximum_value_bytes: state.maximum_value_bytes,
            migrates_from: state.migrates_from.clone(),
        });
    }
    namespaces.sort_by(|left, right| left.namespace.cmp(&right.namespace));
    Ok(PluginStateConfiguration { namespaces })
}

pub(crate) fn validators(
    manifest: &Manifest,
) -> Result<BTreeMap<String, jsonschema::Validator>, AdminError> {
    manifest
        .state
        .iter()
        .map(|state| {
            let validator = jsonschema::options()
                .offline()
                .with_pattern_options(
                    jsonschema::PatternOptions::fancy_regex().backtrack_limit(20_000),
                )
                .build(&state.schema)
                .map_err(|_| AdminError::invalid("插件状态 schema 无效或引用了外部资源"))?;
            Ok((state.namespace.clone(), validator))
        })
        .collect()
}
