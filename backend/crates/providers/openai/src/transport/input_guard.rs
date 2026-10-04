//! GPT 长上下文输入守卫的判定口径。
//!
//! 阈值声明归 `gateway_core::metering::GPT_LONG_CONTEXT_INPUT_TOKEN_THRESHOLD` 一家所有
//! （与长上下文计费共用）；这里只放两件纯逻辑：
//!
//! 1. **模型判定**：按**最终上游模型名**判断是否属于 GPT 系列——去空白、取最后一个 `/`
//!    之后的部分、ASCII 小写，等于 `gpt` 或以 `gpt-` 开头即命中。不看渠道名或公开别名。
//! 2. **输入估算**：宁高不低的口径（ASCII 约 4 字符/token，非 ASCII 约 1.2 token/字符，
//!    `input_image` 按 1500/张，输入条目 12/条，工具定义 24/个）。
//!
//! 估算只服务"是否可能超限"的判定：它刻意不算精确，也不参与计费（计费按上游返回的真实用量）。

use gateway_core::metering::GPT_LONG_CONTEXT_INPUT_TOKEN_THRESHOLD;
use serde_json::{Map, Value};

/// ASCII 字符每 token 的字符数（约 4）。
const ASCII_CHARS_PER_TOKEN: u64 = 4;
/// 非 ASCII 字符的 token 系数（约 1.2）。
const NON_ASCII_TOKENS_PER_CHAR: f64 = 1.2;
/// 每个输入条目的固定开销。
const INPUT_ITEM_TOKENS: u64 = 12;
/// 每张 `input_image` 的固定开销。
const INPUT_IMAGE_TOKENS: u64 = 1_500;
/// 每个工具定义的固定开销。
const TOOL_DEFINITION_TOKENS: u64 = 24;
/// 协议脚手架键：其开销已由"每条目 12 / 每工具 24"覆盖，不再重复按文本计入。
const PROTOCOL_KEYS: &[&str] = &[
    "type",
    "role",
    "name",
    "namespace",
    "call_id",
    "id",
    "status",
    "model",
];

/// 该最终上游模型名是否属于 GPT 系列（守卫只作用于 GPT 系列）。
///
/// 去空白 → 取最后一个 `/` 之后的部分 → ASCII 小写 → 等于 `gpt` 或以 `gpt-` 开头。
#[must_use]
pub fn is_gpt_family_model(upstream_model: &str) -> bool {
    let trimmed = upstream_model.trim();
    let leaf = trimmed.rsplit('/').next().unwrap_or(trimmed);
    let leaf = leaf.to_ascii_lowercase();
    leaf == "gpt" || leaf.starts_with("gpt-")
}

/// 按「宁高不低」口径估算请求正文的输入 token。
///
/// 覆盖 `instructions`、`input` 条目与 `tools`；`input_image` 只记固定开销，
/// 其 `image_url`（可能是很长的 data URL）不计字符，否则会严重高估。
#[must_use]
pub fn estimate_input_tokens(body: &Map<String, Value>) -> u64 {
    let mut tokens = body
        .get("instructions")
        .and_then(Value::as_str)
        .map_or(0, text_tokens);

    if let Some(input) = body.get("input").and_then(Value::as_array) {
        for item in input {
            tokens = tokens.saturating_add(INPUT_ITEM_TOKENS);
            tokens = tokens.saturating_add(value_tokens(item));
        }
    }

    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        tokens = tokens.saturating_add(tool_tokens(tools));
    }

    tokens
}

/// 文本按 ASCII / 非 ASCII 分别向上取整，避免低估。
fn text_tokens(text: &str) -> u64 {
    let mut ascii = 0_u64;
    let mut non_ascii = 0_u64;
    for character in text.chars() {
        if character.is_ascii() {
            ascii += 1;
        } else {
            non_ascii += 1;
        }
    }
    ascii.div_ceil(ASCII_CHARS_PER_TOKEN)
        + (non_ascii as f64 * NON_ASCII_TOKENS_PER_CHAR).ceil() as u64
}

/// 递归累计一个 JSON 值里的文本开销；图像项按固定开销处理且不再深入其内容。
fn value_tokens(value: &Value) -> u64 {
    match value {
        Value::String(text) => text_tokens(text),
        Value::Array(items) => items.iter().map(value_tokens).sum(),
        Value::Object(object) => {
            if is_input_image(object) {
                return INPUT_IMAGE_TOKENS;
            }
            object
                .iter()
                .filter(|(key, _)| !PROTOCOL_KEYS.contains(&key.as_str()))
                .map(|(_, value)| value_tokens(value))
                .sum()
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => 0,
    }
}

/// `input_image` 的两种既有写法：内容块 `type` 或条目 `type`。
fn is_input_image(object: &Map<String, Value>) -> bool {
    object.get("type").and_then(Value::as_str) == Some("input_image")
}

/// 输入守卫的运行时配置。
///
/// **默认关闭**：这是一条新增的拒绝路径，开启前必须由运营显式打开并设定阈值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputGuardConfig {
    /// 是否启用守卫。
    pub enabled: bool,
    /// 允许的最大估算输入 token；超过即拒绝。
    pub threshold_tokens: u64,
}

/// 阈值配置项的允许下限（规范 §4：1000–2_000_000）。
pub const INPUT_GUARD_THRESHOLD_MIN: u64 = 1_000;
/// 阈值配置项的允许上限。
pub const INPUT_GUARD_THRESHOLD_MAX: u64 = 2_000_000;

/// 由管理端设置构造守卫配置时的校验错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputGuardConfigError {
    /// 阈值不在允许范围内。
    ThresholdOutOfRange {
        /// 被拒绝的阈值。
        value: u64,
    },
}

impl InputGuardConfig {
    /// 由管理端设置构造配置。
    ///
    /// # Errors
    ///
    /// 阈值不在 `[INPUT_GUARD_THRESHOLD_MIN, INPUT_GUARD_THRESHOLD_MAX]` 内时返回错误。
    /// 这里**不做静默夹取**：配置越界属运营误操作，应显式失败并让调用方报错，
    /// 否则"看起来配了 500 实际按 1000 生效"这类偏差无处可查。
    pub fn from_settings(
        enabled: bool,
        threshold_tokens: u64,
    ) -> Result<Self, InputGuardConfigError> {
        if !(INPUT_GUARD_THRESHOLD_MIN..=INPUT_GUARD_THRESHOLD_MAX).contains(&threshold_tokens) {
            return Err(InputGuardConfigError::ThresholdOutOfRange {
                value: threshold_tokens,
            });
        }
        Ok(Self {
            enabled,
            threshold_tokens,
        })
    }
}

impl Default for InputGuardConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_tokens: GPT_LONG_CONTEXT_INPUT_TOKEN_THRESHOLD,
        }
    }
}

/// 守卫判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputGuardDecision {
    /// 未开启、非 GPT 系列、压缩请求，或估算未超限：照常发送。
    Allow,
    /// 估算超限：拒绝，并把估算值带出来用于诊断。
    Reject {
        /// 本次估算出的输入 token。
        estimated_tokens: u64,
    },
}

/// 判定该请求是否应被输入守卫拒绝。
///
/// 只作用于**最终上游模型名属于 GPT 系列**的请求；压缩请求必须豁免——压缩本身
/// 携带完整历史，拒绝它会让客户端永远无法把上下文压下来。`is_compaction` 由调用方
/// 传入请求语义里既有的 `compact` 标志（turn metadata 的 `request_kind` 或 input 里的
/// `compaction_trigger`），这里不重复实现那套识别。
#[must_use]
pub fn decide_input_guard(
    config: InputGuardConfig,
    body: &Map<String, Value>,
    upstream_model: &str,
    is_compaction: bool,
) -> InputGuardDecision {
    if !config.enabled || is_compaction || !is_gpt_family_model(upstream_model) {
        return InputGuardDecision::Allow;
    }
    let estimated_tokens = estimate_input_tokens(body);
    if estimated_tokens > config.threshold_tokens {
        InputGuardDecision::Reject { estimated_tokens }
    } else {
        InputGuardDecision::Allow
    }
}

/// 工具定义按个数计；`namespace` 容器内的工具同样各自计入。
fn tool_tokens(tools: &[Value]) -> u64 {
    tools
        .iter()
        .map(|tool| {
            let nested = tool
                .get("tools")
                .and_then(Value::as_array)
                .map_or(0, |inner| tool_tokens(inner));
            TOOL_DEFINITION_TOKENS.saturating_add(nested)
        })
        .sum()
}
