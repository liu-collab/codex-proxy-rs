//! 下游客户端模型请求正文的 Codex 协议兼容

use serde_json::{Map, Value, json};

/// 非官方客户端及跨客户端历史回填的兼容入口；调用方必须已选定 Codex/OAuth 上游
/// 只处理已验证被拒绝的字段，不根据客户端品牌推断整份历史是否合法
pub(super) fn normalize_non_codex_request_body(body: &mut Map<String, Value>) {
    normalize_codex_oauth_options(body);
    let Some(input) = body.get_mut("input").and_then(Value::as_array_mut) else {
        return;
    };
    for item in input {
        let Some(item) = item.as_object_mut() else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("reasoning") {
            continue;
        }
        // SDK 输出项的 status 不属于 Codex reasoning 输入合同；其他项的 status 可能合法
        item.shift_remove("status");
        // Codex 只接受空 content 数组；仅在加密历史仍可回填时去掉冗余明文，
        // 没有加密内容的历史不自动丢弃，保留给上游明确拒绝
        if item
            .get("encrypted_content")
            .and_then(Value::as_str)
            .is_some_and(|content| !content.trim().is_empty())
            && item
                .get("content")
                .and_then(Value::as_array)
                .is_some_and(|content| !content.is_empty())
        {
            item.shift_remove("content");
        }
        // `UUID-序号` 形状的 encrypted_content 是占位值而非可回填密文：删掉它；
        // 该项 id 也是 36 位 UUID 时一并删除（该 id 只标识占位项，回放无意义）。
        if item
            .get("encrypted_content")
            .and_then(Value::as_str)
            .is_some_and(is_placeholder_encrypted_content)
        {
            item.shift_remove("encrypted_content");
            if item
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(is_uuid36)
            {
                item.shift_remove("id");
            }
        }
    }
}

/// `UUID-序号` 形状的占位密文：36 位 UUID 后接 `-` 与纯数字
fn is_placeholder_encrypted_content(value: &str) -> bool {
    let Some((uuid, sequence)) = value.rsplit_once('-') else {
        return false;
    };
    is_uuid36(uuid) && !sequence.is_empty() && sequence.bytes().all(|byte| byte.is_ascii_digit())
}

/// 36 位 UUID（8-4-4-4-12 十六进制）
fn is_uuid36(value: &str) -> bool {
    const DASH_POSITIONS: [usize; 4] = [8, 13, 18, 23];
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        if DASH_POSITIONS.contains(&index) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

fn normalize_codex_oauth_options(body: &mut Map<String, Value>) {
    // Codex OAuth 拒绝这些客户端选项，即使采样参数取默认值
    for field in [
        "top_p",
        "frequency_penalty",
        "presence_penalty",
        "chat_template_kwargs",
        "user",
        "metadata",
        "safety_identifier",
    ] {
        body.shift_remove(field);
    }
    // Responses 自带用量；保留官方流式推理摘要等其他选项
    if let Some(options) = body
        .get_mut("stream_options")
        .and_then(Value::as_object_mut)
    {
        options.shift_remove("include_usage");
    }
    // 只省略没有实际效果的配置，不静默取消客户端要求的截断、停止条件或缓存策略
    if body.get("truncation").and_then(Value::as_str) == Some("disabled") {
        body.shift_remove("truncation");
    }
    if body
        .get("stop_sequences")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        body.shift_remove("stop_sequences");
    }
    if body
        .get("prompt_cache_options")
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty)
    {
        body.shift_remove("prompt_cache_options");
    }
}

/// 补齐 Codex 请求缺省字段并适配已确认不兼容的请求形状，不递归清洗业务正文
///
/// 兼容基准是 Codex Core/Desktop 的模型请求，不是公开 OpenAI Responses API
/// 未知字段继续透传，不能因官方请求结构中没有某个字段就将其列入过滤规则
pub(crate) fn normalize_codex_request_body(body: &mut Map<String, Value>) {
    // 官方 Core/Desktop 显式发送 store=false；仅为缺字段的下游请求补齐，保留显式值
    body.entry("store").or_insert(Value::Bool(false));

    // 公开 Responses API 允许 `input` 为字符串（等价于一条 user 文本消息），
    // 而 Codex 后端只接受条目数组，否则返回 400 "Input must be a list"
    // 这里按官方 ResponseItem::Message 的形状展开；其他非数组类型不猜测语义，交给上游判定
    if let Some(Value::String(text)) = body.get_mut("input") {
        let text = std::mem::take(text);
        body.insert(
            "input".to_owned(),
            json!([{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": text}],
            }]),
        );
    }

    // Codex 上游拒绝显式 message 的 system role；沿用官方客户端的 developer
    // role 承载指令，只转换已确认的消息形状，保留内容与其他字段
    if let Some(input) = body.get_mut("input").and_then(Value::as_array_mut) {
        for item in input {
            let Some(item) = item.as_object_mut() else {
                continue;
            };
            if item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("system")
            {
                item.insert("role".to_owned(), Value::String("developer".to_owned()));
            }
        }
    }

    // Responses 上游统一忽略客户端输出上限，避免请求因上限低于模型下限被拒
    // Codex 也不接受显式采样温度和缓存保留时长
    for field in [
        "max_output_tokens",
        "max_completion_tokens",
        "max_tokens",
        "temperature",
        "prompt_cache_retention",
    ] {
        body.shift_remove(field);
    }

    strip_client_identity_fields(body);
}

/// 剥离顶层客户端身份字段。
///
/// `user` 与 `safety_identifier` 是下游客户端/终端用户的标识，不随请求转发上游；
/// `client_metadata` 内的同名键属于业务 metadata，保持原样。
/// 这是"未知字段继续透传"原则的定向例外：这两个字段已知且只用于标识调用方，
/// 不参与模型行为。
pub(crate) fn strip_client_identity_fields(body: &mut Map<String, Value>) {
    body.remove("user");
    body.remove("safety_identifier");
}
