//! 代理地址的用户输入展开：允许直接粘贴代理商常用的 `主机:端口:用户名:密码`。
//!
//! 代理地址要存库、要脱敏展示、要被账号按 ID 引用，所以规范形态只能是
//! `scheme://[用户名:密码@]主机:端口`；而各家代理商给的是 `主机:端口:用户名:密码`、
//! `用户名:密码@主机:端口` 这类自定义拼串，**同一个拼串既可能是 HTTP 也可能是 SOCKS5**。
//! 协议因此由调用方显式选择，这里只做形状展开与最终校验，不猜协议：猜错不会立刻报错，
//! 只会在真正出站时变成难查的连接失败。
//!
//! 展开后仍走 [`OutboundProxy::parse`] 的同一份校验，避免出现第二套格式规则。

use std::fmt::Write as _;

use gateway_core::account::OutboundProxy;

/// 可选协议；与 [`OutboundProxy::parse`] 接受的白名单一致。
pub(super) const PROXY_PROTOCOLS: [&str; 4] = ["http", "https", "socks5", "socks5h"];

/// 地址形状不合法时的提示。不回显输入值：代理 URL 里含账号密码。
const ADDRESS_HELP: &str = "代理地址格式不合法：填写完整 URL（scheme://用户名:密码@主机:端口），或粘贴「主机:端口:用户名:密码」并选择协议";

/// 解析代理地址输入。
///
/// - 已带 `scheme://` 的完整 URL 自行声明协议，忽略 `protocol`；
/// - 否则用 `protocol` 展开简写，缺少协议时明确要求选择而不是默认一个。
pub(super) fn parse_proxy_address(
    raw: &str,
    protocol: Option<&str>,
) -> Result<OutboundProxy, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("代理 URL 不能为空".to_owned());
    }
    if raw.contains("://") {
        return OutboundProxy::parse(raw).map_err(|_| ADDRESS_HELP.to_owned());
    }
    let Some(protocol) = protocol.map(str::trim).filter(|value| !value.is_empty()) else {
        return Err("请选择代理协议（HTTP / SOCKS5），或直接填写完整 URL".to_owned());
    };
    let protocol = protocol.to_ascii_lowercase();
    if !PROXY_PROTOCOLS.contains(&protocol.as_str()) {
        return Err(format!(
            "不支持的代理协议 {protocol}；仅支持 {}",
            PROXY_PROTOCOLS.join(" / ")
        ));
    }
    let url = expand_shorthand(raw, &protocol).ok_or_else(|| ADDRESS_HELP.to_owned())?;
    OutboundProxy::parse(&url).map_err(|_| ADDRESS_HELP.to_owned())
}

/// `主机:端口`、`主机:端口:用户名:密码`、`用户名:密码@主机:端口` → 规范 URL。
///
/// 先按冒号分段（代理商最常见的四段写法），只有分不出 1 段或 3 段时才回退到
/// `用户名:密码@主机:端口`：这样凭据里带 `@` 的四段写法不会被切错位置。
fn expand_shorthand(raw: &str, protocol: &str) -> Option<String> {
    if let Some((host, tail)) = split_host(raw) {
        match tail.split(':').collect::<Vec<_>>().as_slice() {
            [port] => return Some(format!("{protocol}://{host}:{port}")),
            [port, user, password] => {
                return Some(format!(
                    "{protocol}://{}:{}@{host}:{port}",
                    encode_userinfo(user),
                    encode_userinfo(password)
                ));
            }
            _ => {}
        }
    }
    let (userinfo, endpoint) = raw.rsplit_once('@')?;
    if userinfo.is_empty() || !is_host_port(endpoint) {
        return None;
    }
    let credentials = match userinfo.split_once(':') {
        Some((user, password)) => {
            format!("{}:{}", encode_userinfo(user), encode_userinfo(password))
        }
        None => encode_userinfo(userinfo),
    };
    Some(format!("{protocol}://{credentials}@{endpoint}"))
}

/// `主机:端口` 形状（允许 IPv6 方括号写法），且不允许多余字段。
fn is_host_port(value: &str) -> bool {
    split_host(value).is_some_and(|(_, tail)| !tail.is_empty() && !tail.contains(':'))
}

/// 拆出主机与端口之后的剩余字段；方括号写法保留在主机里，便于原样拼回 URL。
fn split_host(raw: &str) -> Option<(&str, &str)> {
    if raw.starts_with('[') {
        let close = raw.find(']')?;
        if close == 1 {
            return None;
        }
        let host = &raw[..=close];
        let tail = raw[close + 1..].strip_prefix(':')?;
        Some((host, tail))
    } else {
        let (host, tail) = raw.split_once(':')?;
        (!host.is_empty()).then_some((host, tail))
    }
}

/// userinfo 里的保留字符必须百分号编码；未保留字符保持原样。
///
/// 代理商密码里常见的 `@ : / ?` 直接拼进 URL 会把地址切错位置，必须编码后才能交给
/// `OutboundProxy::parse`。
fn encode_userinfo(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}
