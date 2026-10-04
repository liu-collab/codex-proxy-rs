//! `local_compaction` 线格式的严格性回归。
//!
//! 判据来自规范：网关本地压缩摘要用固定前缀 `cpr-local-v1:` 标记，还原时严格校验
//! Base64 与 UTF-8，非法编码必须在出站前被拒绝，不能发出损坏数据。

use base64::{Engine as _, engine::general_purpose::STANDARD};
use provider_openai::transport::local_compaction::{
    LOCAL_COMPACTION_PREFIX, LocalCompactionError, decode_local_compaction,
    encode_local_compaction, is_local_compaction, restore_local_compaction_history,
};
use serde_json::json;

#[test]
fn round_trip_preserves_non_ascii_summary_text() {
    let summary = "摘要：读取 src/main.rs 并修复 --flag 解析\nsecond line";
    let encoded = encode_local_compaction(summary);

    assert!(encoded.starts_with(LOCAL_COMPACTION_PREFIX));
    assert!(is_local_compaction(&encoded));
    assert_eq!(decode_local_compaction(&encoded).expect("decode"), summary);
}

#[test]
fn plain_text_is_not_a_local_compaction_payload() {
    assert!(!is_local_compaction("ordinary upstream summary"));
    assert_eq!(
        decode_local_compaction("ordinary upstream summary"),
        Err(LocalCompactionError::NotLocalCompaction)
    );
    // 前缀必须出现在开头：正文里提到同名字样不算标记。
    assert!(!is_local_compaction("see cpr-local-v1: for details"));
}

#[test]
fn empty_payload_is_rejected() {
    assert_eq!(
        decode_local_compaction(LOCAL_COMPACTION_PREFIX),
        Err(LocalCompactionError::Empty)
    );
}

#[test]
fn malformed_base64_is_rejected() {
    assert_eq!(
        decode_local_compaction(&format!("{LOCAL_COMPACTION_PREFIX}not*base64!")),
        Err(LocalCompactionError::InvalidBase64)
    );
}

#[test]
fn non_utf8_payload_is_rejected() {
    // 0xFF/0xFE 不是合法 UTF-8 起始字节，但完全可以是合法 Base64 载荷。
    let encoded = format!(
        "{LOCAL_COMPACTION_PREFIX}{}",
        STANDARD.encode([0xff_u8, 0xfe])
    );
    assert_eq!(
        decode_local_compaction(&encoded),
        Err(LocalCompactionError::InvalidUtf8)
    );
}

#[test]
fn restore_replaces_marked_items_and_keeps_other_history() {
    let summary = "关键结论：已修复 --flag 解析";
    let mut input = vec![
        json!({"type":"message","role":"user","content":"hello"}),
        json!({
            "type":"reasoning",
            "summary":[{"type":"summary_text","text":"readable for the client"}],
            "encrypted_content": encode_local_compaction(summary),
        }),
        json!({"type":"function_call","call_id":"call_1","name":"lookup","arguments":"{}"}),
    ];

    assert!(restore_local_compaction_history(&mut input).expect("restore"));
    assert_eq!(
        input[1],
        json!({
            "type":"message",
            "role":"user",
            "content":[{
                "type":"input_text",
                "text": format!("<conversation_summary>\n{summary}\n</conversation_summary>"),
            }],
        })
    );
    // 未命中的历史项原样保留。
    assert_eq!(input[0]["type"], "message");
    assert_eq!(input[2]["type"], "function_call");
}

#[test]
fn restore_reports_no_change_and_rejects_malformed_payloads() {
    let mut without_marker =
        vec![json!({"type":"reasoning","summary":[],"encrypted_content":"real-upstream-cipher"})];
    assert!(!restore_local_compaction_history(&mut without_marker).expect("no marker"));

    let mut broken = vec![json!({
        "type":"reasoning",
        "encrypted_content": format!("{LOCAL_COMPACTION_PREFIX}not*base64!"),
    })];
    assert_eq!(
        restore_local_compaction_history(&mut broken),
        Err(LocalCompactionError::InvalidBase64)
    );
}
