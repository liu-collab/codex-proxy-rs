//! PostgreSQL 包含搜索共用的 SQL 字面量转义，大小写规则由调用方决定

pub(super) fn literal_contains_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len().saturating_add(2));
    escaped.push('%');
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.push('%');
    escaped
}
