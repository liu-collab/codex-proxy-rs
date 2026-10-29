//! 验证 OpenAI 配置解析、路径派生与上游地址约束

use std::{path::Path, time::Duration};

use provider_openai::config::{MAX_STREAM_MAX_RETRIES, OpenAiConfig};

#[test]
fn openai_config_preserves_optional_routing_header_values() {
    for (document, expected) in [
        (r#"{}"#, (None, None)),
        (
            r#"{"residency":null,"account_routing_override":null}"#,
            (None, None),
        ),
        (
            r#"{"residency":"us","account_routing_override":"us"}"#,
            (Some("us"), Some("us")),
        ),
        (
            r#"{"residency":"custom-region","account_routing_override":"custom-route"}"#,
            (Some("custom-region"), Some("custom-route")),
        ),
        (
            r#"{"residency":"","account_routing_override":"NO_CONSTRAINT"}"#,
            (Some(""), Some("NO_CONSTRAINT")),
        ),
    ] {
        let config: OpenAiConfig = serde_json::from_str(document).unwrap();
        assert_eq!(
            (
                config.residency.as_deref(),
                config.account_routing_override.as_deref()
            ),
            expected,
            "{document}",
        );
    }
}

#[test]
fn openai_config_derives_identity_secret_from_runtime_data_dir() {
    let mut config = valid_config();
    config
        .resolve_and_validate(Path::new("/srv/gateway/runtime-data"))
        .expect("valid OpenAI config");

    assert!(format!("{config:?}").contains("/srv/gateway/runtime-data/identity_hmac_secret"));
}

#[test]
fn openai_config_restricts_upstream_base_url_to_https_or_loopback_http() {
    for base_url in [
        "http://internal.example.com/backend-api",
        "http://10.0.0.7/backend-api",
        "https://chatgpt.com/backend-api?debug=1",
        "https://user:pass@chatgpt.com/backend-api",
        "https://chatgpt.com/backend-api#fragment",
        "ftp://chatgpt.com/backend-api",
    ] {
        let mut config = valid_config();
        config.api.base_url = base_url.to_owned();
        assert!(
            config
                .resolve_and_validate(Path::new("/srv/gateway"))
                .is_err(),
            "expected {base_url} to be rejected"
        );
    }

    for base_url in [
        "https://chatgpt.com/backend-api",
        "http://127.0.0.1:8080/backend-api",
        "http://localhost:8080/backend-api",
        "http://[::1]:8080/backend-api",
    ] {
        let mut config = valid_config();
        config.api.base_url = base_url.to_owned();
        assert!(
            config
                .resolve_and_validate(Path::new("/srv/gateway"))
                .is_ok(),
            "expected {base_url} to be accepted"
        );
    }
}

#[test]
fn openai_stream_retry_budget_uses_the_official_hard_cap() {
    let mut config = OpenAiConfig::default();
    config.stream_max_retries = MAX_STREAM_MAX_RETRIES + 1;

    assert_eq!(
        config.stream_max_retries(),
        u32::try_from(MAX_STREAM_MAX_RETRIES).expect("retry cap fits u32")
    );
}

/// 守卫默认关闭、阈值取共用声明；越界在启动校验阶段被拒绝（不静默夹取）。
#[test]
fn openai_input_guard_is_disabled_by_default_and_rejects_out_of_range_thresholds() {
    let config = OpenAiConfig::default();
    assert!(!config.input_guard().enabled, "守卫必须默认关闭");
    assert_eq!(config.input_guard().threshold_tokens, 272_000);

    for threshold in [0, 999, 2_000_001, u64::MAX] {
        let mut config: OpenAiConfig = serde_json::from_value(serde_json::json!({
            "input_guard": {"enabled": true, "threshold_tokens": threshold}
        }))
        .expect("parse OpenAI config");
        assert_eq!(
            config.resolve_and_validate(Path::new("/srv/gateway/runtime-data")),
            Err(provider_openai::OpenAiConfigError::InvalidField(
                "openai.input_guard.threshold_tokens"
            )),
            "阈值 {threshold} 越界必须拒绝启动"
        );
    }
}

#[test]
fn websocket_pool_idle_timeout_accepts_existing_configs_and_explicit_overrides() {
    for (idle_timeout_ms, expected) in [(None, 300_000), (Some(600_000), 600_000)] {
        let mut document = serde_json::json!({
            "ws_pool": {
                "enabled": true,
                "max_age_ms": 3_300_000,
                "max_connecting": 8,
                "stream_idle_timeout_ms": 300_000
            }
        });
        if let Some(idle_timeout_ms) = idle_timeout_ms {
            document["ws_pool"]["idle_timeout_ms"] = serde_json::json!(idle_timeout_ms);
        }
        let mut config: OpenAiConfig = serde_json::from_value(document).unwrap();
        config
            .resolve_and_validate(Path::new("/srv/gateway"))
            .unwrap();
        assert_eq!(
            config.websocket_pool_config().idle_timeout,
            Duration::from_millis(expected)
        );
    }
}

#[test]
fn openai_input_guard_settings_reach_the_provider_config() {
    for threshold in [1_000, 272_000, 2_000_000] {
        let mut config: OpenAiConfig = serde_json::from_value(serde_json::json!({
            "input_guard": {"enabled": true, "threshold_tokens": threshold}
        }))
        .expect("parse OpenAI config");
        config
            .resolve_and_validate(Path::new("/srv/gateway/runtime-data"))
            .expect("valid guard settings");

        let guard = config.input_guard();
        assert!(guard.enabled);
        assert_eq!(guard.threshold_tokens, threshold);
    }
}

#[test]
fn websocket_pool_idle_timeout_rejects_zero() {
    let mut config = valid_config();
    config.ws_pool.idle_timeout_ms = 0;
    assert!(
        config
            .resolve_and_validate(Path::new("/srv/gateway"))
            .is_err()
    );
}

fn valid_config() -> OpenAiConfig {
    OpenAiConfig::default()
}
