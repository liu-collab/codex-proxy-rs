//! 网关本地压缩摘要的线格式。
//!
//! 网关侧压缩产出的摘要会在后续请求里被客户端原样回带。为了让**所有模型**都能把它
//! 还原成普通 user 摘要消息，网关用自己的固定前缀标记这类载荷：`cpr-local-v1:` 之后
//! 是摘要正文的 Base64 编码。
//!
//! 解码必须严格：前缀缺失、载荷为空、Base64 非法或解码结果不是 UTF-8，都返回错误，
//! 由调用方按 InvalidRequest/NotSent 拒绝——绝不把损坏的摘要发给上游。

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

/// 网关本地压缩摘要的固定前缀。
pub const LOCAL_COMPACTION_PREFIX: &str = "cpr-local-v1:";

/// 网关本地压缩载荷的解析错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCompactionError {
    /// 不是网关本地摘要：缺少固定前缀。
    NotLocalCompaction,
    /// 前缀存在但载荷为空。
    Empty,
    /// 载荷不是合法 Base64。
    InvalidBase64,
    /// 载荷解码成功但不是 UTF-8。
    InvalidUtf8,
}

/// 把摘要正文编码成网关本地压缩载荷。
#[must_use]
pub fn encode_local_compaction(summary: &str) -> String {
    format!(
        "{LOCAL_COMPACTION_PREFIX}{}",
        STANDARD.encode(summary.as_bytes())
    )
}

/// 该值是否携带网关本地压缩标记。
///
/// 只认**恰好以固定前缀开头**的值；不做 trim，避免把上游正文里出现的同名字样
/// 误判成网关载荷。
#[must_use]
pub fn is_local_compaction(value: &str) -> bool {
    value.starts_with(LOCAL_COMPACTION_PREFIX)
}

/// 严格解析网关本地压缩载荷，返回摘要正文。
///
/// # Errors
///
/// 缺少前缀、载荷为空、Base64 非法或结果不是 UTF-8 时返回对应错误。
pub fn decode_local_compaction(value: &str) -> Result<String, LocalCompactionError> {
    let Some(encoded) = value.strip_prefix(LOCAL_COMPACTION_PREFIX) else {
        return Err(LocalCompactionError::NotLocalCompaction);
    };
    if encoded.is_empty() {
        return Err(LocalCompactionError::Empty);
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| LocalCompactionError::InvalidBase64)?;
    String::from_utf8(bytes).map_err(|_| LocalCompactionError::InvalidUtf8)
}

/// 把历史里带网关标记的 reasoning 项还原成普通 user 摘要消息。
///
/// 命中条件：`type == "reasoning"` 且 `encrypted_content` 以固定前缀开头。
/// 返回值表示是否发生了替换；未命中任何项时返回 `false`，调用方无需清理续接状态。
///
/// # Errors
///
/// `encrypted_content` 携带前缀但载荷为空、Base64 非法或不是 UTF-8 时返回对应错误；
/// 调用方必须据此按 InvalidRequest/NotSent 拒绝整条请求，绝不把损坏的摘要发上游。
pub fn restore_local_compaction_history(input: &mut [Value]) -> Result<bool, LocalCompactionError> {
    let mut restored = false;
    for item in input.iter_mut() {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        if object.get("type").and_then(Value::as_str) != Some("reasoning") {
            continue;
        }
        let Some(payload) = object.get("encrypted_content").and_then(Value::as_str) else {
            continue;
        };
        if !is_local_compaction(payload) {
            continue;
        }
        let summary = decode_local_compaction(payload)?;
        *item = compaction_summary_message(&summary);
        restored = true;
    }
    Ok(restored)
}

/// 还原后的 user 摘要消息形状。
///
/// 与既有还原约定一致：摘要正文包在 `<conversation_summary>` 标签里，便于后续
/// 摘要请求（其提示词按该标签识别前一次压缩）把早期历史继续带下去。
fn compaction_summary_message(summary: &str) -> Value {
    json!({
        "type": "message",
        "role": "user",
        "content": [{
            "type": "input_text",
            "text": format!("<conversation_summary>\n{summary}\n</conversation_summary>"),
        }],
    })
}
