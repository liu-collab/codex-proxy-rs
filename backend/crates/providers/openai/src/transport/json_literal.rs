//! 字节保真的 JSON 字符串字面量替换。
//!
//! 搜索、图像等 raw JSON 端点的正文只允许做最小差异改写：重复键、字段顺序、原始空白、
//! 大整数精度与 base64 内容都必须逐字保留。`serde_json` 的整体重编码会丢掉重复键与原始
//! 空白，因此这里用一次线性扫描定位目标路径上的字符串字面量，只替换该字面量的字节区间，
//! 其余字节原样拼回。

use std::ops::Range;

use serde_json::Value;

/// 把 `path` 指向的**已存在**字符串值替换为 `replacement`。
///
/// * `path` 是对象键序列，例如 `["client_metadata", "x-codex-installation-id"]`。
/// * 目标不存在、目标不是字符串、或 JSON 结构不可信时返回 `None`。调用方必须保留原文：
///   宁可整条不改写，也不发出半改写的正文（失败关闭）。
/// * 成功时只有该字符串字面量的字节被替换，其余字节逐字保留。
///
/// 同一路径出现重复键时按最先出现的匹配处理：重复键本身按原样保留，不合并、不排序。
pub fn replace_existing_string_at_path(
    input: &[u8],
    path: &[&str],
    replacement: &str,
) -> Option<Vec<u8>> {
    let span = find_string_value(input, path)?;
    let encoded = serde_json::to_vec(&Value::String(replacement.to_owned())).ok()?;
    let mut output = Vec::with_capacity(input.len() + encoded.len());
    output.extend_from_slice(&input[..span.start]);
    output.extend_from_slice(&encoded);
    output.extend_from_slice(&input[span.end..]);
    Some(output)
}

/// 定位 `path` 指向的字符串字面量（含两侧引号）的字节区间。
fn find_string_value(input: &[u8], path: &[&str]) -> Option<Range<usize>> {
    // 先整体校验：顶层必须是对象，且必须能被完整消费（拒绝截断、多余内容与非法顶层结构）。
    // 只有整份文档结构可信时才做替换，避免把字节拼接进一份已经损坏的正文。
    let mut validator = Cursor::new(input);
    validator.skip_whitespace();
    if validator.peek()? != b'{' {
        return None;
    }
    validator.skip_value()?;
    validator.skip_whitespace();
    if validator.pos != input.len() {
        return None;
    }

    let mut cursor = Cursor::new(input);
    cursor.skip_whitespace();
    if cursor.peek()? != b'{' {
        return None;
    }
    cursor.pos += 1;
    find_in_object(&mut cursor, path)
}

/// 在当前对象内按 `path` 剩余段查找；返回字符串字面量（含引号）的区间。
fn find_in_object(cursor: &mut Cursor<'_>, path: &[&str]) -> Option<Range<usize>> {
    let (segment, rest) = path.split_first()?;
    loop {
        cursor.skip_whitespace();
        match cursor.peek()? {
            b'}' => return None,
            b'"' => {}
            _ => return None,
        }
        let (key_range, _) = cursor.read_string()?;
        let key = std::str::from_utf8(&cursor.bytes[key_range]).ok()?;
        cursor.skip_whitespace();
        if cursor.peek()? != b':' {
            return None;
        }
        cursor.pos += 1;
        cursor.skip_whitespace();
        if key == *segment {
            if rest.is_empty() {
                // 目标就是这一层，其值必须是字符串字面量，否则失败关闭。
                if cursor.peek()? != b'"' {
                    return None;
                }
                let literal_start = cursor.pos;
                let (_, literal_end) = cursor.read_string()?;
                return Some(literal_start..literal_end);
            }
            if cursor.peek()? != b'{' {
                return None;
            }
            cursor.pos += 1;
            if let Some(found) = find_in_object(cursor, rest) {
                return Some(found);
            }
        } else {
            cursor.skip_value()?;
        }
        cursor.skip_whitespace();
        match cursor.peek()? {
            b',' => cursor.pos += 1,
            b'}' => return None,
            _ => return None,
        }
    }
}

/// 最小的 JSON 扫描游标：只做结构遍历与字符串字面量定位，不做值解析。
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.bytes.get(self.pos) {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// 读取一个字符串字面量，返回 `(内容区间, 字面量结束位置)`；`pos` 必须指向 `"`。
    fn read_string(&mut self) -> Option<(Range<usize>, usize)> {
        if self.peek()? != b'"' {
            return None;
        }
        let content_start = self.pos + 1;
        let mut index = content_start;
        loop {
            match *self.bytes.get(index)? {
                b'"' => {
                    self.pos = index + 1;
                    return Some((content_start..index, self.pos));
                }
                // 转义序列整段跳过：`\uXXXX` 的十六进制位不会是 `"` 或 `\`，
                // 因此跳过后继续按普通字节消费即可。
                b'\\' => index += 2,
                _ => index += 1,
            }
        }
    }

    /// 跳过任意 JSON 值（调用前可含前导空白）。
    fn skip_value(&mut self) -> Option<()> {
        self.skip_whitespace();
        match self.peek()? {
            b'"' => {
                self.read_string()?;
                Some(())
            }
            b'{' => {
                self.pos += 1;
                loop {
                    self.skip_whitespace();
                    if self.peek()? == b'}' {
                        self.pos += 1;
                        return Some(());
                    }
                    self.read_string()?;
                    self.skip_whitespace();
                    if self.peek()? != b':' {
                        return None;
                    }
                    self.pos += 1;
                    self.skip_value()?;
                    self.skip_whitespace();
                    match self.peek()? {
                        b',' => self.pos += 1,
                        b'}' => {
                            self.pos += 1;
                            return Some(());
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                self.pos += 1;
                loop {
                    self.skip_whitespace();
                    if self.peek()? == b']' {
                        self.pos += 1;
                        return Some(());
                    }
                    self.skip_value()?;
                    self.skip_whitespace();
                    match self.peek()? {
                        b',' => self.pos += 1,
                        b']' => {
                            self.pos += 1;
                            return Some(());
                        }
                        _ => return None,
                    }
                }
            }
            _ => {
                // 数字 / true / false / null：消费到下一个结构分隔符。
                let start = self.pos;
                while let Some(byte) = self.peek() {
                    if matches!(byte, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r') {
                        break;
                    }
                    self.pos += 1;
                }
                (self.pos > start).then_some(())
            }
        }
    }
}
