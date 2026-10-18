//! 校验插件实例配置、敏感字段与能力绑定，生成安全的配置错误

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use gateway_admin::model::{
    AdminError,
    plugins::{instances::PluginInstance, state::PluginStateConfiguration},
};
use gateway_plugin_sdk::{Capability, Manifest, Stage};
use secrecy::ExposeSecret as _;

/// 制品摘要绑定不可变 schema；缓存不持有实例配置与敏感值
#[derive(Default)]
pub(crate) struct ConfigurationSchemas {
    validators: Mutex<BTreeMap<String, Arc<jsonschema::Validator>>>,
}

impl ConfigurationSchemas {
    fn get(
        &self,
        artifact: &str,
        schema: &serde_json::Value,
    ) -> Result<Arc<jsonschema::Validator>, AdminError> {
        if let Some(validator) = self
            .validators
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(artifact)
            .cloned()
        {
            return Ok(validator);
        }
        // 编译不占缓存锁；schema 只能引用同一 JSON 文档，不读取网络或文件
        let validator = Arc::new(
            jsonschema::options()
                .offline()
                .with_pattern_options(
                    jsonschema::PatternOptions::fancy_regex().backtrack_limit(20_000),
                )
                .build(schema)
                .map_err(|_| AdminError::invalid("插件配置 schema 无效或引用外部资源"))?,
        );
        let mut validators = self
            .validators
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(existing) = validators.get(artifact) {
            return Ok(Arc::clone(existing));
        }
        // 与制品兼容性缓存保持同一容量，历史版本不无限延长编译结果寿命
        if validators.len() >= 128 {
            validators.pop_first();
        }
        validators.insert(artifact.to_owned(), Arc::clone(&validator));
        Ok(validator)
    }
}

pub(super) fn validate(
    instance: &PluginInstance,
    manifest: &Manifest,
    schemas: &ConfigurationSchemas,
) -> Result<(serde_json::Value, PluginStateConfiguration), AdminError> {
    if instance.enabled && !instance.trusted_process {
        return Err(AdminError::invalid("插件制品尚未接受安装"));
    }
    let (configuration, ready) = prepare_configuration(
        instance,
        &manifest.configuration_schema,
        &manifest.secret_fields,
        schemas,
    )?;
    if instance.enabled && !ready {
        return Err(AdminError::invalid("请填写插件必填配置"));
    }
    let mut bindings = BTreeSet::new();
    for binding in &instance.bindings {
        let resolved = crate::contribution::resolve(manifest, binding)?;
        let capability = resolved.capability;
        if matches!(
            capability,
            Capability::Management | Capability::CommandLine | Capability::Maintenance
        ) {
            return Err(AdminError::invalid("管理页面、命令行与维护无需功能绑定"));
        }
        let stage: Stage = serde_json::from_value(serde_json::Value::String(binding.stage.clone()))
            .map_err(|_| AdminError::invalid("插件调用阶段不合法"))?;
        if (capability != Capability::Observer && !bindings.insert((capability, stage)))
            || !resolved.declaration.stages.contains(&stage)
        {
            return Err(AdminError::invalid("插件能力或阶段未声明，或者重复绑定"));
        }
        if capability != Capability::Observer && binding.event.is_some() {
            return Err(AdminError::invalid("事件订阅只能用于观察绑定"));
        }
        if capability != Capability::FrontendAuthentication && !binding.identity_bindings.is_empty()
        {
            return Err(AdminError::invalid("身份映射只能用于客户端认证绑定"));
        }
    }
    Ok((
        configuration,
        crate::callback::private_state::configuration(manifest)?,
    ))
}

pub(super) fn configuration_ready(
    instance: &PluginInstance,
    schema: &serde_json::Value,
    secret_fields: &BTreeSet<String>,
    schemas: &ConfigurationSchemas,
) -> Result<bool, AdminError> {
    prepare_configuration(instance, schema, secret_fields, schemas).map(|(_, ready)| ready)
}

fn prepare_configuration(
    instance: &PluginInstance,
    schema: &serde_json::Value,
    secret_fields: &BTreeSet<String>,
    schemas: &ConfigurationSchemas,
) -> Result<(serde_json::Value, bool), AdminError> {
    let mut configuration = instance
        .configuration
        .as_object()
        .cloned()
        .ok_or_else(|| AdminError::invalid("插件配置必须是对象"))?;
    for (name, value) in &instance.secrets {
        if !secret_fields.contains(name) || configuration.contains_key(name) {
            return Err(AdminError::invalid("插件敏感字段必须通过独立 secret 配置"));
        }
        configuration.insert(
            name.clone(),
            serde_json::Value::String(value.expose_secret().into()),
        );
    }
    if secret_fields
        .iter()
        .any(|field| instance.configuration.get(field).is_some())
    {
        return Err(AdminError::invalid("普通配置不能包含声明的敏感字段"));
    }
    let configuration = serde_json::Value::Object(configuration);
    let schema = schemas.get(&instance.artifact_sha256, schema)?;
    let mut ready = true;
    for error in schema.iter_errors(&configuration) {
        // 待配置只代表缺少必填值，不能借停用保存类型错误或非法配置
        if matches!(
            error.kind(),
            jsonschema::error::ValidationErrorKind::Required { .. }
        ) {
            ready = false;
        } else {
            return Err(configuration_error(&error));
        }
    }
    Ok((configuration, ready))
}

fn configuration_error(error: &jsonschema::ValidationError<'_>) -> AdminError {
    use jsonschema::error::ValidationErrorKind;

    // 校验库的 Display 会包含原值，敏感配置也参与校验，只返回字段路径和静态原因
    let mut path = error.instance_path().to_string();
    let reason = match error.kind() {
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            if let Some(name) = unexpected.first() {
                path.push('/');
                path.push_str(&name.replace('~', "~0").replace('/', "~1"));
            }
            "目标版本不支持此字段"
        }
        ValidationErrorKind::Type { .. } => "类型不匹配",
        ValidationErrorKind::Enum { .. } => "不在允许的选项中",
        _ => "不符合字段要求",
    };
    let path: String = path
        .chars()
        .filter(|value| !value.is_control())
        .take(128)
        .collect();
    let field = if path.is_empty() { "根对象" } else { &path };
    AdminError::invalid(format!("插件配置 {field}：{reason}"))
}
