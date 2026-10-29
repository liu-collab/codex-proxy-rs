//! 从已解析的 Responses 对象提取安全证据，不改变正文或协议处理

use gateway_core::diagnostics::{TraceContext, body_fingerprint, json_fingerprint};
use gateway_protocol::openai::events::extract_usage;
use serde_json::{Map, Value, json};

use crate::transport::protocol::responses::CodexResponsesRequest;

const INPUT_ITEM_SAMPLE: usize = 16;
const INPUT_BLOCK_SIZE: usize = 16;
const MAX_INPUT_BLOCKS: usize = 32;

pub(crate) fn record_request(
    trace: &TraceContext,
    stage: &'static str,
    body: &Map<String, Value>,
    transport: &'static str,
    headers: Option<&reqwest::header::HeaderMap>,
) {
    if !trace.is_enabled() {
        return;
    }
    let fields = [
        "model",
        "client_metadata",
        "prompt_cache_key",
        "instructions",
        "tools",
        "reasoning",
        "text",
        "service_tier",
        "tool_choice",
        "parallel_tool_calls",
        "previous_response_id",
    ]
    .into_iter()
    .map(|key| {
        (
            key.to_owned(),
            body.get(key).map_or(Value::Null, json_fingerprint),
        )
    })
    .collect::<Map<_, _>>();
    let input = body.get("input");
    let items = input.and_then(Value::as_array);
    let item_hashes = items
        .into_iter()
        .flatten()
        .take(INPUT_ITEM_SAMPLE)
        .map(json_fingerprint)
        .collect::<Vec<_>>();
    let block_hashes = items
        .into_iter()
        .flat_map(|items| items.chunks(INPUT_BLOCK_SIZE))
        .take(MAX_INPUT_BLOCKS)
        .map(json_fingerprint)
        .collect::<Vec<_>>();
    let correlation = headers.map(|headers| {
        [
            "session_id", "thread_id", "x-codex-session-id", "x-codex-thread-id",
            "x-codex-window-id", "x-codex-turn-state", "x-codex-client-metadata",
        ].into_iter().map(|name| {
            let values = headers.get_all(name);
            (name.to_owned(), json!({
                "values": values.iter().take(4).map(|value| body_fingerprint(value.as_bytes())).collect::<Vec<_>>(),
                "omittedValues": values.iter().count().saturating_sub(4),
            }))
        }).collect::<Map<_, _>>()
    });
    trace.record_evidence(stage, json!({
        "transport": transport,
        "correlation": correlation,
        "fields": fields,
        "input": {
            "fingerprint": input.map(json_fingerprint),
            "items": items.map(Vec::len),
            "itemHashes": item_hashes,
            "blockSize": INPUT_BLOCK_SIZE,
            "blockHashes": block_hashes,
            "omittedItems": items.map(|items| items.len().saturating_sub(INPUT_BLOCK_SIZE * MAX_INPUT_BLOCKS)),
        },
    }));
}

pub(crate) fn request_state(request: &CodexResponsesRequest) -> Value {
    let fingerprint = |value: Option<&str>| value.map(|value| body_fingerprint(value.as_bytes()));
    json!({
        "session": fingerprint(request.client_session_id.as_deref()),
        "clientConversation": fingerprint(request.client_conversation_id.as_deref()),
        "conversation": fingerprint(request.local_conversation_id.as_deref()),
        "window": fingerprint(request.codex_window_id.as_deref()),
        "turnState": fingerprint(request.turn_state.as_deref()),
        "metadataTurnState": request.client_metadata().and_then(|metadata| metadata.get("x-codex-turn-state")).map(json_fingerprint),
        "previousResponse": fingerprint(request.previous_response_id()),
        "promptCacheKey": request.body().get("prompt_cache_key").map(json_fingerprint),
    })
}

pub(crate) fn record_parse_gap(
    trace: &TraceContext,
    reason: &'static str,
    body: Option<&[u8]>,
    event_type: Option<&str>,
) {
    if trace.is_enabled() {
        trace.record_evidence(
            "provider.response.parse_gap",
            json!({
                "reason": reason,
                "body": body.map(body_fingerprint),
                "eventType": event_type.map(|kind| body_fingerprint(kind.as_bytes())),
            }),
        );
    }
}

pub(crate) fn record_response_usage(trace: &TraceContext, event_type: &str, value: &Value) {
    if !trace.is_enabled() {
        return;
    }
    let response = value.get("response").unwrap_or(value);
    let usage = response.get("usage");
    let source = usage.unwrap_or(response);
    let parsed = extract_usage(response);
    trace.record_evidence("provider.response.usage", json!({
        "eventType": event_type,
        "responseId": response.get("id").map(json_fingerprint),
        "boundary": "terminal_event_observed",
        "usageState": match usage { None => "missing", Some(Value::Object(_)) => "object", Some(_) => "invalid" },
        "fields": {
            "inputTokens": token_field(source, &["/input_tokens", "/prompt_tokens"]),
            "outputTokens": token_field(source, &["/output_tokens", "/completion_tokens"]),
            "cachedInputTokens": token_field(source, &["/input_tokens_details/cached_tokens", "/prompt_tokens_details/cached_tokens", "/cached_tokens"]),
            "cacheWriteInputTokens": token_field(source, &["/input_tokens_details/cache_write_tokens", "/prompt_tokens_details/cache_write_tokens", "/cache_write_tokens"]),
            "reasoningOutputTokens": token_field(source, &["/output_tokens_details/reasoning_tokens", "/completion_tokens_details/reasoning_tokens", "/reasoning_tokens"]),
            "totalTokens": token_field(source, &["/total_tokens"]),
        },
        "parsedUsage": parsed,
    }));
}

fn token_field(source: &Value, paths: &[&str]) -> Value {
    // 各已知位置分别记录存在性与数值；取舍仍由共享 usage parser 决定
    Value::Array(
        paths
            .iter()
            .map(|path| match source.pointer(path) {
                Some(value) => match value.as_u64() {
                    Some(value) => json!({"state": "value", "source": path, "value": value}),
                    None => json!({"state": "invalid", "source": path}),
                },
                None => json!({"state": "missing", "source": path}),
            })
            .collect(),
    )
}
