//! 账号内身份伪名的回归。
//!
//! 判据来自规范 §2：会话、线程、轮次、窗口等键做**账号内**稳定伪名；同一原值在不同
//! 载体上映射为同一伪名；`workspaces` 只处理工作区根与远端 URL 值，label 等原样保留。

use provider_openai::transport::identity_pseudonym::{
    IDENTITY_ID_KEYS, IdentityPseudonym, pseudonymize_identity_fields, pseudonymize_workspaces,
};
use serde_json::{Map, Value, json};

fn object(value: Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

#[test]
fn pseudonyms_are_stable_within_an_account_and_unlinkable_across_accounts() {
    let first = IdentityPseudonym::for_account("installation-a");
    let same = IdentityPseudonym::for_account("installation-a");
    let other = IdentityPseudonym::for_account("installation-b");

    assert_eq!(first.of("client-session"), same.of("client-session"));
    assert_ne!(first.of("client-session"), other.of("client-session"));
    assert_ne!(first.of("client-session"), "client-session");
}

#[test]
fn pseudonyms_are_uuid_shaped_and_value_only() {
    let pseudonym = IdentityPseudonym::for_account("installation-a");
    let session = pseudonym.of("client-session");
    assert_eq!(session.len(), 36);
    assert_eq!(session.matches('-').count(), 4);

    // 只按原值派生：同一个原值无论出现在哪个键、哪条载体上都是同一个伪名。
    let mut metadata = object(json!({
        "session_id": "shared",
        "thread_id": "shared",
        "x-codex-window-id": "client-window",
    }));
    assert!(pseudonymize_identity_fields(&mut metadata, &pseudonym));
    assert_eq!(metadata["session_id"], metadata["thread_id"]);
    assert_eq!(metadata["session_id"], json!(pseudonym.of("shared")));
    assert_eq!(
        metadata["x-codex-window-id"],
        json!(pseudonym.of("client-window"))
    );
}

#[test]
fn identity_fields_cover_every_documented_key_class() {
    for class in [
        "session_id",
        "thread_id",
        "parent_thread_id",
        "root_thread_id",
        "forked_from_thread_id",
        "conversation_id",
        "turn_id",
        "parent_turn_id",
        "root_turn_id",
        "window_id",
        "context_window_id",
        "prompt_cache_key",
    ] {
        assert!(
            IDENTITY_ID_KEYS.contains(&class) || class == "prompt_cache_key",
            "{class} 必须参与伪名"
        );
        let mut metadata = object(json!({class: "original"}));
        assert!(
            pseudonymize_identity_fields(&mut metadata, &IdentityPseudonym::for_account("a")),
            "{class}"
        );
        assert_ne!(metadata[class], json!("original"), "{class}");
    }
}

#[test]
fn empty_and_non_string_values_are_left_alone() {
    let mut metadata = object(json!({
        "session_id": "",
        "thread_id": 7,
        "future_field": "keep",
    }));
    assert!(!pseudonymize_identity_fields(
        &mut metadata,
        &IdentityPseudonym::for_account("a")
    ));
    assert_eq!(metadata["session_id"], json!(""));
    assert_eq!(metadata["thread_id"], json!(7));
    assert_eq!(metadata["future_field"], json!("keep"));
}

#[test]
fn workspaces_pseudonymize_roots_and_remote_urls_but_keep_labels() {
    let pseudonym = IdentityPseudonym::for_account("installation-a");
    let mut metadata = object(json!({
        "installation_id": "client-installation",
        "workspaces": {
            "/tmp/project": {
                "label": "café",
                "commit": "deadbeef",
                "associated_remote_urls": ["https://github.com/example/repo"],
                "future": {"keep": true}
            }
        }
    }));
    assert!(pseudonymize_workspaces(&mut metadata, &pseudonym));

    let workspaces = metadata["workspaces"].as_object().expect("workspaces");
    assert_eq!(workspaces.len(), 1);
    let (root, workspace) = workspaces.iter().next().expect("workspace root");
    assert_eq!(root, &pseudonym.of("/tmp/project"));
    assert_eq!(workspace["label"], json!("café"));
    assert_eq!(workspace["commit"], json!("deadbeef"));
    assert_eq!(
        workspace["associated_remote_urls"][0],
        json!(pseudonym.of("https://github.com/example/repo"))
    );
    assert_eq!(workspace["future"], json!({"keep": true}));
    // 工作区根之外的键不动。
    assert_eq!(metadata["installation_id"], json!("client-installation"));
}

#[test]
fn empty_workspaces_are_not_rewritten() {
    let mut metadata = object(json!({"workspaces": {}}));
    assert!(!pseudonymize_workspaces(
        &mut metadata,
        &IdentityPseudonym::for_account("a")
    ));
    assert_eq!(metadata["workspaces"], json!({}));
}
