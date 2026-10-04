//! 审核覆盖映射的回归。
//!
//! 判据来自规范 §5：识别 `x-openai-subagent: review`、正文
//! `client_metadata["x-openai-subagent"]` 与 `x-codex-turn-metadata.subagent_kind`；
//! 按声明模型名判族（`deepseek*`→`deepseek-flash`/max、
//! `mimo*`→`mimo-v2.6-flash`/max、`gpt*`→`gpt-5.6-luna` 缺失时 `terraform`/`terra`/xhigh），
//! 且不得猜一个清单里不存在的模型名。

use provider_openai::transport::review_override::{is_review_request, review_override};
use serde_json::{Map, json};

fn models(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn request(
    client_metadata: Option<serde_json::Value>,
    turn_metadata: Option<&str>,
) -> (
    Map<String, serde_json::Value>,
    Map<String, serde_json::Value>,
) {
    let mut body = Map::from_iter([("model".to_owned(), json!("gpt-5.6-sol"))]);
    if let Some(client_metadata) = client_metadata {
        body.insert("client_metadata".to_owned(), client_metadata);
    }
    let context = turn_metadata.map_or_else(Map::new, |turn_metadata| {
        Map::from_iter([("turn_metadata".to_owned(), json!(turn_metadata))])
    });
    (body, context)
}

#[test]
fn review_requests_are_identified_on_every_documented_channel() {
    // HTTP 头 `x-openai-subagent: review` 由 API 层注入正文 client_metadata。
    let (body, context) = request(
        Some(json!({"x-openai-subagent": "review"})),
        Some(r#"{"request_kind":"review"}"#),
    );
    assert!(is_review_request(&body, &context));

    // WebSocket 帧只有 turn metadata 这一路。
    let (body, context) = request(None, Some(r#"{"subagent_kind":"review"}"#));
    assert!(is_review_request(&body, &context));

    // 兼容客户端的扁平键：正文 client_metadata，无头无 turn metadata。
    let (body, context) = request(Some(json!({"x-openai-subagent": "review"})), None);
    assert!(is_review_request(&body, &context));

    // 正文里裸的 turn metadata 也要认（没有 API 层上下文时的回退）。
    let (mut body, context) = request(None, None);
    body.insert(
        "x-codex-turn-metadata".to_owned(),
        json!(r#"{"subagent_kind":"review"}"#),
    );
    assert!(is_review_request(&body, &context));
}

#[test]
fn non_review_subagents_and_blank_values_are_not_review_requests() {
    for (client_metadata, turn_metadata) in [
        (Some(json!({"x-openai-subagent": "worker"})), None),
        (None, Some(r#"{"subagent_kind":"thread_spawn"}"#)),
        (Some(json!({"x-openai-subagent": "  "})), None),
        (Some(json!({"x-openai-subagent": 7})), None),
        // client_metadata 不是 object 时不能从别处推断。
        (Some(json!("opaque-client-value")), None),
        (None, None),
    ] {
        let (body, context) = request(client_metadata, turn_metadata);
        assert!(
            !is_review_request(&body, &context),
            "body={body:?} context={context:?}"
        );
    }
}

#[test]
fn turn_metadata_subagent_kind_wins_over_the_flat_client_metadata_key() {
    // 两处都声明时以官方 turn metadata 为准，与 CodexResponsesRequest::subagent_kind 一致。
    let (body, context) = request(
        Some(json!({"x-openai-subagent": "worker"})),
        Some(r#"{"subagent_kind":"review"}"#),
    );
    assert!(is_review_request(&body, &context));

    let (body, context) = request(
        Some(json!({"x-openai-subagent": "review"})),
        Some(r#"{"subagent_kind":"worker"}"#),
    );
    assert!(!is_review_request(&body, &context));
}

#[test]
fn deepseek_and_mimo_families_use_their_flash_review_models() {
    let available = models(&["deepseek-flash", "mimo-v2.6-flash"]);
    for declared in ["deepseek-v4-flash", "DeepSeek-V3.2", "vendor/deepseek-v3"] {
        let over = review_override(declared, &available).expect("deepseek 覆盖");
        assert_eq!(over.upstream_model, "deepseek-flash");
        assert_eq!(over.reasoning_effort, "max");
    }
    for declared in ["mimo-v2.6", "MiMo-V2-Flash"] {
        let over = review_override(declared, &available).expect("mimo 覆盖");
        assert_eq!(over.upstream_model, "mimo-v2.6-flash");
        assert_eq!(over.reasoning_effort, "max");
    }
}

#[test]
fn gpt_family_prefers_luna_then_falls_back_in_order() {
    let with_luna = models(&["gpt-5.6-luna", "terraform", "terra"]);
    assert_eq!(
        review_override("gpt-5.6-sol", &with_luna)
            .expect("gpt 覆盖")
            .upstream_model,
        "gpt-5.6-luna"
    );

    // luna 缺失 → terraform 优先于 terra。
    let without_luna = models(&["terraform", "terra"]);
    assert_eq!(
        review_override("gpt-5.6-sol", &without_luna)
            .expect("gpt 覆盖")
            .upstream_model,
        "terraform"
    );

    // 只剩 terra 时用它。
    let only_terra = models(&["terra"]);
    assert_eq!(
        review_override("gpt-5.6-sol", &only_terra)
            .expect("gpt 覆盖")
            .upstream_model,
        "terra"
    );

    // GPT 族的强度一律 xhigh。
    assert_eq!(
        review_override("gpt-5.6-sol", &with_luna)
            .expect("gpt 覆盖")
            .reasoning_effort,
        "xhigh"
    );
}

#[test]
fn gpt_family_without_any_review_model_is_not_guessed() {
    // 清单里一个候选都没有：返回 None，绝不编一个可能不存在的模型名。
    assert_eq!(
        review_override("gpt-5.6-sol", &models(&["gpt-5.6-sol"])),
        None
    );
    assert_eq!(review_override("gpt-5.6-sol", &[]), None);
}

#[test]
fn unrelated_families_are_not_rewritten() {
    let available = models(&["gpt-5.6-luna", "deepseek-flash", "mimo-v2.6-flash"]);
    for declared in ["grok-4.5", "o3", "claude-sonnet-4.5", "gemini-3-pro"] {
        assert_eq!(
            review_override(declared, &available),
            None,
            "{declared} 不属于受覆盖的族"
        );
    }
}
