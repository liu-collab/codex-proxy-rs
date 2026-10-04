//! `json_literal::replace_existing_string_at_path` 的字节保真回归。
//!
//! 判据来自规范：raw JSON 端点只允许替换 JSON 字符串字面量，其余字节（重复键、字段顺序、
//! 空白、大整数精度、base64 内容）必须逐字保留；任何无法确定的情况都失败关闭。

use provider_openai::transport::json_literal::replace_existing_string_at_path;

/// 目标外的每个字节都必须原样保留：重复键、紧凑空白、大整数、base64、未知嵌套字段。
#[test]
fn replaces_only_the_target_literal_and_keeps_every_other_byte() {
    let input = concat!(
        r#"{"id":"session-1","prompt":"hi","prompt":"duplicate","client_metadata":{"#,
        r#""x-codex-installation-id":"downstream-install","future":{"a":[1,2,{"b":null}]}},"#,
        r#""n":9007199254740993,"blob":"iVBORw0KGgo=","tail":"x"}"#,
    );
    let output = replace_existing_string_at_path(
        input.as_bytes(),
        &["client_metadata", "x-codex-installation-id"],
        "account-install",
    )
    .expect("目标存在且是字符串");

    let text = String::from_utf8(output).expect("输出仍是 UTF-8");
    // 只发生了一次替换，其余字节完全一致（含重复键 prompt 与大整数 9007199254740993）。
    assert_eq!(text, input.replace("downstream-install", "account-install"));
    assert!(text.contains(r#""prompt":"hi","prompt":"duplicate""#));
    assert!(text.contains("9007199254740993"));
    assert!(text.contains(r#""blob":"iVBORw0KGgo=""#));
}

#[test]
fn replacement_is_json_escaped() {
    let input = br#"{"client_metadata":{"x-codex-installation-id":"old"}}"#;
    let output = replace_existing_string_at_path(
        input,
        &["client_metadata", "x-codex-installation-id"],
        "a\"b\\c\nd",
    )
    .expect("目标存在且是字符串");

    assert_eq!(
        String::from_utf8(output).expect("UTF-8"),
        r#"{"client_metadata":{"x-codex-installation-id":"a\"b\\c\nd"}}"#
    );
}

#[test]
fn missing_path_is_fail_closed() {
    let input = br#"{"client_metadata":{}}"#;
    assert!(
        replace_existing_string_at_path(
            input,
            &["client_metadata", "x-codex-installation-id"],
            "x"
        )
        .is_none(),
        "目标键缺失时必须保留原文"
    );
    assert!(replace_existing_string_at_path(input, &["missing"], "x").is_none());
}

#[test]
fn non_string_target_is_fail_closed() {
    let input = br#"{"installation_id":42}"#;
    assert!(
        replace_existing_string_at_path(input, &["installation_id"], "x").is_none(),
        "目标不是字符串时不得改写"
    );
    let nested = br#"{"client_metadata":{"x-codex-installation-id":{"a":1}}}"#;
    assert!(
        replace_existing_string_at_path(
            nested,
            &["client_metadata", "x-codex-installation-id"],
            "x"
        )
        .is_none()
    );
}

/// 同名键出现在别的路径时不受影响：只改目标路径。
#[test]
fn same_key_at_another_path_is_untouched() {
    let input = concat!(
        r#"{"installation_id":"top","nested":{"installation_id":"inner"},"#,
        r#""client_metadata":{"x-codex-installation-id":"meta"}}"#,
    );
    let output = replace_existing_string_at_path(
        input.as_bytes(),
        &["client_metadata", "x-codex-installation-id"],
        "new",
    )
    .expect("目标存在");

    assert_eq!(
        String::from_utf8(output).expect("UTF-8"),
        concat!(
            r#"{"installation_id":"top","nested":{"installation_id":"inner"},"#,
            r#""client_metadata":{"x-codex-installation-id":"new"}}"#,
        )
    );
}

/// 相邻字符串里的转义序列（`\"`、`\u007b`）不能干扰定位。
#[test]
fn escapes_in_neighbouring_strings_do_not_confuse_the_scanner() {
    let input = concat!(
        r#"{"a":"he said \"{\" ok","#,
        r#""client_metadata":{"x-codex-installation-id":"old"},"b":"tail"}"#,
    );
    let output = replace_existing_string_at_path(
        input.as_bytes(),
        &["client_metadata", "x-codex-installation-id"],
        "new",
    )
    .expect("目标存在");

    let text = String::from_utf8(output).expect("UTF-8");
    assert!(text.contains(r#""a":"he said \"{\" ok""#));
    assert!(text.contains(r#""x-codex-installation-id":"new""#));
    assert!(text.ends_with(r#""b":"tail"}"#));
}

#[test]
fn malformed_json_is_fail_closed() {
    assert!(replace_existing_string_at_path(br#"{"a":"b""#, &["a"], "x").is_none());
    assert!(replace_existing_string_at_path(b"not json", &["a"], "x").is_none());
    assert!(replace_existing_string_at_path(br#"["a"]"#, &["a"], "x").is_none());
}
