//! 审核请求（`x-openai-subagent: review` 一类）的模型与推理强度覆盖。
//!
//! 审核请求由客户端以独立子代理身份发起，模型名往往落在不适合审核的族上；
//! 网关按**声明的模型名**判族，把它换到审核用的模型与强度：
//!
//! | 声明模型族 | 覆盖模型 | 推理强度 |
//! |---|---|---|
//! | `deepseek*` | `deepseek-flash` | `max` |
//! | `mimo*` | `mimo-v2.6-flash` | `max` |
//! | `gpt*` | 优先 `gpt-5.6-luna`，缺失时 `terraform`，再缺失 `terra` | `xhigh` |
//!
//! 这里只回答"这是不是审核请求"与"该换成什么"，不负责路由
//! （覆盖必须在选号前生效，由调用方保证）。

use gateway_protocol::openai::codex_responses_request_semantics;
use serde_json::{Map, Value};

use super::client::openai_subagent_from_metadata;

/// 审核子代理的标识值，同时用于 `x-openai-subagent` 与 turn metadata 的 `subagent_kind`。
pub const REVIEW_SUBAGENT_KIND: &str = "review";

/// 审核请求的模型与推理强度覆盖结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOverride {
    /// 覆盖后的上游模型名。
    pub upstream_model: String,
    /// 覆盖后的推理强度。
    pub reasoning_effort: &'static str,
}

/// GPT 审核模型的优先顺序：缺失时依次回退。
const GPT_REVIEW_MODELS: &[&str] = &["gpt-5.6-luna", "terraform", "terra"];

/// 判定请求是否为审核子代理请求。
///
/// 规范列出的三个通道归一到两处事实：HTTP 头 `x-openai-subagent` 由 API 层注入正文
/// `client_metadata`，与兼容客户端自己写的 `client_metadata["x-openai-subagent"]` 是
/// 同一个扁平键；官方 Codex 形态是 `x-codex-turn-metadata` 里的 `subagent_kind`
/// （WebSocket 帧只有后者）。两处都声明时以 turn metadata 为准，与
/// `CodexResponsesRequest::subagent_kind` 的优先级保持一致。
#[must_use]
pub fn is_review_request(body: &Map<String, Value>, context: &Map<String, Value>) -> bool {
    codex_responses_request_semantics(body, context)
        .subagent_kind
        .or_else(|| openai_subagent_from_metadata(body.get("client_metadata")))
        .is_some_and(|kind| kind.trim() == REVIEW_SUBAGENT_KIND)
}

/// 按声明模型名给出审核覆盖；不属于任何受覆盖的族时返回 `None`。
///
/// `available_models` 是当前可用于审核的模型清单：只有在清单里确实存在的候选才会
/// 被选中。一个候选都没有时返回 `None`——**绝不猜一个可能不存在的模型名**，
/// 因为把审核请求路由到不存在的模型会让审核整体失败。
#[must_use]
pub fn review_override(
    declared_model: &str,
    available_models: &[String],
) -> Option<ReviewOverride> {
    let declared = declared_model.to_ascii_lowercase();
    let has = |candidate: &str| available_models.iter().any(|model| model == candidate);

    if declared.contains("deepseek") {
        return Some(ReviewOverride {
            upstream_model: "deepseek-flash".to_owned(),
            reasoning_effort: "max",
        });
    }
    if declared.contains("mimo") {
        return Some(ReviewOverride {
            upstream_model: "mimo-v2.6-flash".to_owned(),
            reasoning_effort: "max",
        });
    }
    if declared.contains("gpt") {
        let upstream_model = GPT_REVIEW_MODELS
            .iter()
            .copied()
            .find(|candidate| has(candidate))?;
        return Some(ReviewOverride {
            upstream_model: upstream_model.to_owned(),
            reasoning_effort: "xhigh",
        });
    }
    None
}
