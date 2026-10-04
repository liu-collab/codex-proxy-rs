//! 输入守卫判定口径的回归：模型名分类与"宁高不低"的估算。
//!
//! 判据来自规范 §4：判定按**最终上游模型名**（去空白 → 最后一个 `/` 之后 → ASCII 小写
//! → `gpt` 或 `gpt-` 前缀）；估算按 ASCII 4 字符/token、非 ASCII 1.2 token/字符、
//! `input_image` 1500/张、条目 12/条、工具定义 24/个。

use provider_openai::transport::input_guard::{
    INPUT_GUARD_THRESHOLD_MAX, INPUT_GUARD_THRESHOLD_MIN, InputGuardConfig, InputGuardConfigError,
    InputGuardDecision, decide_input_guard, estimate_input_tokens, is_gpt_family_model,
};
use serde_json::{Map, json};

fn body(value: serde_json::Value) -> Map<String, serde_json::Value> {
    value.as_object().expect("request object").clone()
}

#[test]
fn model_classification_uses_the_final_upstream_leaf() {
    for model in [
        "gpt-5.4",
        "gpt",
        "  GPT-5.4  ",
        "openai/gpt-5.4",
        "vendor/GPT-6.1-sol",
    ] {
        assert!(is_gpt_family_model(model), "{model} 应命中 GPT 系列");
    }
    for model in [
        "o3",
        "codex-mini",
        "claude-gpt",
        "gptx-5",
        "deepseek-flash",
        "",
        "vendor/",
    ] {
        assert!(!is_gpt_family_model(model), "{model} 不应命中 GPT 系列");
    }
    // 渠道名/别名不参与判定：只有最后一个 `/` 之后的部分算。
    assert!(!is_gpt_family_model("gpt-channel/grok-4.5"));
}

#[test]
fn ascii_text_is_counted_at_four_characters_per_token() {
    // "abcdefgh" = 8 ASCII 字符 → 2 token；再加一个输入条目 12。
    let tokens = estimate_input_tokens(&body(json!({
        "input": [{"type": "message", "role": "user", "content": "abcdefgh"}],
    })));
    assert_eq!(tokens, 2 + 12);
}

#[test]
fn non_ascii_text_is_rounded_up() {
    // 两个中文字符 → ceil(2 × 1.2) = 3 token；再加条目 12。
    let tokens = estimate_input_tokens(&body(json!({
        "input": [{"type": "message", "role": "user", "content": "中文"}],
    })));
    assert_eq!(tokens, 3 + 12);
}

#[test]
fn instructions_are_counted() {
    // 12 个 ASCII 字符 → 3 token。
    let tokens = estimate_input_tokens(&body(json!({"instructions": "abcdefghijkl"})));
    assert_eq!(tokens, 3);
}

#[test]
fn input_image_is_a_fixed_cost_and_its_payload_is_not_counted_as_text() {
    let huge_payload = "A".repeat(100_000);
    let tokens = estimate_input_tokens(&body(json!({
        "input": [{
            "type": "message",
            "role": "user",
            "content": [
                {"type": "input_text", "text": "abcd"},
                {"type": "input_image", "image_url": format!("data:image/png;base64,{huge_payload}")},
            ],
        }],
    })));
    // 条目 12 + 文本 1 + 图像 1500；10 万字符的 data URL 不计入。
    assert_eq!(tokens, 12 + 1 + 1_500);
}

#[test]
fn tools_are_counted_per_definition_including_namespace_containers() {
    let flat = estimate_input_tokens(&body(json!({
        "tools": [{"type": "function", "name": "a"}, {"type": "function", "name": "b"}],
    })));
    assert_eq!(flat, 2 * 24);

    let namespaced = estimate_input_tokens(&body(json!({
        "tools": [{
            "type": "namespace",
            "name": "functions",
            "tools": [{"type": "function", "name": "shell"}, {"type": "function", "name": "patch"}],
        }],
    })));
    assert_eq!(namespaced, 24 + 2 * 24);
}

#[test]
fn guard_is_disabled_by_default() {
    let config = InputGuardConfig::default();
    assert!(!config.enabled, "守卫必须默认关闭");
    assert_eq!(config.threshold_tokens, 272_000);
    // 即便正文巨大、模型是 GPT 系列，默认配置也不拒绝。
    let huge = body(json!({
        "input": [{"type": "message", "role": "user", "content": "a".repeat(4_000_000)}],
    }));
    assert_eq!(
        decide_input_guard(config, &huge, "gpt-5.4", false),
        InputGuardDecision::Allow
    );
}

#[test]
fn settings_validation_accepts_the_documented_range_and_rejects_outliers() {
    // 边界值本身合法。
    for threshold in [
        INPUT_GUARD_THRESHOLD_MIN,
        272_000,
        INPUT_GUARD_THRESHOLD_MAX,
    ] {
        let config = InputGuardConfig::from_settings(true, threshold).expect("阈值合法");
        assert!(config.enabled);
        assert_eq!(config.threshold_tokens, threshold);
    }
    // 越界一律显式失败，并回带被拒的值（不做静默夹取）。
    for threshold in [
        INPUT_GUARD_THRESHOLD_MIN - 1,
        INPUT_GUARD_THRESHOLD_MAX + 1,
        0,
        u64::MAX,
    ] {
        assert_eq!(
            InputGuardConfig::from_settings(false, threshold),
            Err(InputGuardConfigError::ThresholdOutOfRange { value: threshold })
        );
    }
}

#[test]
fn guard_only_applies_to_gpt_family_models() {
    let config = InputGuardConfig {
        enabled: true,
        threshold_tokens: 100,
    };
    let long = body(json!({
        "input": [{"type": "message", "role": "user", "content": "a".repeat(4_000)}],
    }));
    assert!(matches!(
        decide_input_guard(config, &long, "gpt-5.4", false),
        InputGuardDecision::Reject { .. }
    ));
    for model in ["deepseek-flash", "mimo-v2.6-flash", "o3"] {
        assert_eq!(
            decide_input_guard(config, &long, model, false),
            InputGuardDecision::Allow,
            "{model} 不属于 GPT 系列，不得被守卫拒绝"
        );
    }
}

#[test]
fn guard_exempts_compaction_requests() {
    let config = InputGuardConfig {
        enabled: true,
        threshold_tokens: 100,
    };
    let long = body(json!({
        "input": [{"type": "message", "role": "user", "content": "a".repeat(4_000)}],
    }));
    assert_eq!(
        decide_input_guard(config, &long, "gpt-5.4", true),
        InputGuardDecision::Allow,
        "压缩请求携带完整历史，必须豁免，否则客户端永远压不下上下文"
    );
}

#[test]
fn guard_allows_up_to_and_including_the_threshold() {
    let config = InputGuardConfig {
        enabled: true,
        threshold_tokens: 14,
    };
    // 8 个 ASCII 字符 = 2 token，加一个条目 12 → 恰好 14：不超限。
    let at_threshold = body(json!({
        "input": [{"type": "message", "role": "user", "content": "abcdefgh"}],
    }));
    assert_eq!(
        decide_input_guard(config, &at_threshold, "gpt-5.4", false),
        InputGuardDecision::Allow
    );
    // 再多一个字符就超限（9 字符 → 3 token + 12 = 15）。
    let over = body(json!({
        "input": [{"type": "message", "role": "user", "content": "abcdefghi"}],
    }));
    assert_eq!(
        decide_input_guard(config, &over, "gpt-5.4", false),
        InputGuardDecision::Reject {
            estimated_tokens: 15
        }
    );
}

#[test]
fn empty_body_estimates_zero_and_is_monotonic() {
    assert_eq!(estimate_input_tokens(&body(json!({}))), 0);
    let small = estimate_input_tokens(&body(json!({
        "input": [{"type": "message", "role": "user", "content": "aaaa"}],
    })));
    let large = estimate_input_tokens(&body(json!({
        "input": [{"type": "message", "role": "user", "content": "a".repeat(4_000)}],
    })));
    assert!(large > small, "更长的历史必须估出更多 token");
}
