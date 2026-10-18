//! 诊断文件按认证与观测合同移除敏感字段，保留错误事实并明确导出截断

use regex::Regex;
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::LazyLock;

const REDACTED: &str = "<redacted>";
const MAX_TEXT: usize = 256 * 1024;
static INLINE: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    [
        (r"(?i)\b((?:set-cookie|cookie2?|(?:proxy-)?authorization|x-api-key|x-oai-is(?:-update)?|x-oai-attestation)\s*:\s*)[^\r\n]+", "${1}<redacted>"),
        (r#"(?i)\b(Bearer|Basic)\s+[^\s"'<>;,]+"#, "${1} <redacted>"),
        // 官方 TokenData 的 access_token / id_token 为 JWT，不按猜测的令牌前缀过滤普通文本
        (r"\beyJ[\w-]+\.[\w-]+\.[\w-]+\b", "<redacted>"),
        (r"(?i)([a-z][a-z0-9+.-]*://)[^\s/@]+@", "${1}<redacted>@"),
        (r#"(?i)\b((?:(?:personal[_-]?)?access[_-]?token|refresh[_-]?token|id[_-]?token|(?:(?:openai|x)[_-]?)?api[_-]?key|(?:current[_-]?|new[_-]?)?password|authorization|proxy[_-]?authorization|x[_-]?oai[_-]?(?:is(?:[_-]?update)?|attestation))["']?\s*[=:]\s*)("[^"\r\n]*"|'[^'\r\n]*'|[^\s&,;]+)"#, "${1}<redacted>"),
    ].into_iter().map(|(pattern, replacement)| (Regex::new(pattern).expect("static credential pattern"), replacement)).collect()
});

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    credential_fields: usize,
    personal_fields: usize,
    inline_credentials: usize,
    truncated_values: usize,
}

struct Redactor {
    report: Report,
    bytes: usize,
    nodes: usize,
}

pub(super) fn redact(mut value: Value) -> Value {
    let mut redactor = Redactor {
        report: Report::default(),
        bytes: 8 * 1024 * 1024,
        nodes: 100_000,
    };
    // 先处理有界文件信息，正文耗尽预算也不能删除 schema、目标或既有缺口
    let mut error = value["error"].take();
    let mut requests = value["requests"].take();
    redactor.visit(&mut value, 0);
    redactor.visit(&mut error, 0);
    redactor.visit(&mut requests, 0);
    value["error"] = error;
    value["requests"] = requests;
    if redactor.report.truncated_values > 0 {
        // 缺口清单可能也触及预算，顶层统计始终保留以免截断变成静默丢失
        value["exportTruncated"] = json!(true);
        if !value["availability"]["gaps"].is_array() {
            value["availability"] = json!({"gaps": []});
        }
        value["availability"]["gaps"]
            .as_array_mut()
            .expect("gap array")
            .push(json!({
                "section": "export", "reason": "export_size_limit",
            }));
    }
    value["redaction"] = json!(redactor.report);
    value
}

fn normalized(key: &str) -> String {
    key.chars()
        .filter(|character| !matches!(character, '_' | '-'))
        .flat_map(char::to_lowercase)
        .collect()
}

// OAuth 字段来自 Provider credential schema 与官方 Codex TokenData；API Key 来自
// CodexCprApiKeyExportAccount / AuthDotJson；密码来自管理登录合同，头部来自传输身份合同
// 不用 token、secret、credential 等泛称推断敏感性，避免删除业务错误上下文
fn secret(key: &str) -> bool {
    matches!(
        key,
        "accesstoken"
            | "personalaccesstoken"
            | "refreshtoken"
            | "idtoken"
            | "apikey"
            | "openaiapikey"
            | "password"
            | "currentpassword"
            | "newpassword"
            | "authorization"
            | "proxyauthorization"
            | "xapikey"
            | "cookie"
            | "cookie2"
            | "setcookie"
            | "xoaiis"
            | "xoaiisupdate"
            | "xoaiattestation"
    )
}

impl Redactor {
    fn omitted(&mut self) -> Value {
        self.report.truncated_values += 1;
        json!({"omitted": "export_size_limit"})
    }

    fn text(&mut self, text: &str) -> String {
        let mut result = text.to_owned();
        for (pattern, replacement) in INLINE.iter() {
            self.report.inline_credentials += pattern.find_iter(&result).count();
            result = pattern.replace_all(&result, *replacement).into_owned();
        }
        result
    }

    fn visit(&mut self, value: &mut Value, depth: usize) {
        if self.nodes == 0 || self.bytes == 0 || depth > 32 {
            *value = self.omitted();
            return;
        }
        self.nodes -= 1;
        // 预算同时计入缩进和容器分隔符，避免深层 JSON 的下载体积显著超过正文预算
        self.bytes = self.bytes.saturating_sub(24 + depth * 4);
        match value {
            Value::String(text) => {
                if text.trim_start().starts_with('{') || text.trim_start().starts_with('[') {
                    if text.len() > MAX_TEXT {
                        *value = self.omitted();
                        return;
                    }
                    if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                        *value = parsed;
                        self.visit(value, depth + 1);
                        return;
                    }
                }
                let cleaned = self.text(text);
                let limit = MAX_TEXT.min(self.bytes / 2);
                if cleaned.len() > limit {
                    let boundary = cleaned.floor_char_boundary(limit);
                    *value = json!({"text": &cleaned[..boundary], "omittedBytes": cleaned.len() - boundary});
                    self.report.truncated_values += 1;
                } else {
                    *text = cleaned;
                }
                self.bytes = self.bytes.saturating_sub(value.to_string().len());
            }
            Value::Array(items) => {
                // 头部也可能以二元组承载，不能只识别对象字段名
                if items.len() == 2
                    && items[0]
                        .as_str()
                        .is_some_and(|name| secret(&normalized(name)))
                {
                    items[1] = json!(REDACTED);
                    self.report.credential_fields += 1;
                }
                let original_len = items.len();
                let mut kept = 0;
                for item in items.iter_mut().take(4096) {
                    if self.nodes == 0 || self.bytes == 0 {
                        break;
                    }
                    self.visit(item, depth + 1);
                    kept += 1;
                }
                if kept < original_len {
                    items.truncate(kept);
                    items.push(json!({"omittedItems": original_len - kept}));
                    self.report.truncated_values += 1;
                }
            }
            Value::Object(object) => {
                if object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| secret(&normalized(name)))
                    && object.contains_key("value")
                {
                    object.insert("value".to_owned(), json!(REDACTED));
                    self.report.credential_fields += 1;
                }
                let fields = std::mem::take(object);
                let total = fields.len();
                for (key, mut item) in fields {
                    if object.len() >= 4096 || self.nodes == 0 || self.bytes == 0 {
                        break;
                    }
                    let normalized = normalized(&key);
                    if secret(&normalized) {
                        item = json!(REDACTED);
                        self.report.credential_fields += 1;
                    } else if matches!(
                        normalized.as_str(),
                        "email"
                            | "accountemail"
                            | "clientip"
                            | "accountname"
                            | "accountnotes"
                            | "credentialname"
                            | "clientapikeyname"
                            | "accountlabel"
                    ) {
                        item = json!(REDACTED);
                        self.report.personal_fields += 1;
                    } else {
                        self.visit(&mut item, depth + 1);
                    }
                    self.bytes = self.bytes.saturating_sub(key.len());
                    object.insert(self.text(&key), item);
                }
                if object.len() < total {
                    object.insert("_omittedFields".to_owned(), json!(total - object.len()));
                    self.report.truncated_values += 1;
                }
            }
            _ => {}
        }
    }
}
