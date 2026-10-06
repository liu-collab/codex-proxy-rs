//! 下游客户端对 Codex Core/Desktop 请求协议的兼容处理
//! 账号身份保护、会话规范化和 HTTP 传输规则由对应职责模块维护

mod body;
mod grok;
mod headers;

use serde_json::{Map, Value};

pub(super) use body::normalize_codex_request_body;

/// 跨模型历史清洗：按**最终上游模型族**执行，与账号类型无关。
///
/// 规范 §6-1 的适用范围是"GPT 系列出站前"，§6 末条又要求清洗覆盖 HTTP/SSE、
/// WebSocket、API Key 与 OAuth 四条路径——两者合起来的含义是：**凡是出站到 GPT 系列
/// 模型就清洗，不看用哪类凭据**；非 GPT 系列不套用 GPT 的形状约束（API Key 账号出站到
/// 自有模型时保持原样）。
pub(crate) fn normalize_universal_history_cleanup(
    body: &mut Map<String, Value>,
    upstream_model: &str,
) {
    if !crate::transport::input_guard::is_gpt_family_model(upstream_model) {
        return;
    }
    body::normalize_non_codex_request_body(body);
}

/// Codex/OAuth 专属的下游兼容清洗：只处理已确认由 Codex/OAuth 上游拒绝的形状
/// （客户端标记内容的 xAI 改写等）。跨模型历史清洗见 `normalize_universal_history_cleanup`。
pub(crate) fn normalize_selected_codex_downstream_body(
    body: &mut Map<String, Value>,
    context: &Map<String, Value>,
) {
    grok::normalize_request_body(body, context);
}

pub(super) fn is_non_codex_request_header(name: &str) -> bool {
    headers::is_non_codex_request_header(name) || grok::is_client_header(name)
}
