//! 会话、线程、轮次、窗口与工作区身份的账号内稳定伪名。
//!
//! 下游把这些身份同时放在 HTTP 头、正文 `client_metadata` 扁平键与内嵌
//! `x-codex-turn-metadata` JSON 里。出站前它们全部换成本账号的伪名：
//!
//! - **账号内稳定**：同一个账号里同一个原值永远得到同一个伪名，上游回放与会话连续性
//!   不受影响。
//! - **跨账号不可关联**：伪名以账号的安装身份为种子，同一原值在不同账号下得到不同伪名。
//! - **跨载体一致**：伪名只按原值派生，不按键名或载体派生，因此规范要求的"同一原值在
//!   头、`turnMetadata`、`client_metadata` 上映射为同一伪名"由构造本身保证，头与正文
//!   不会各改各的。
//!
//! 只处理身份键与工作区根：`workspaces` 的对象键（工作区根）与
//! `associated_remote_urls` 值按账号伪名，`label`、提交号等其余内容原样保留。

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// 键名感知的身份 id 键：值是需要伪名化的身份。
///
/// 覆盖规范 §2 的 session、thread、parent_thread、root_thread、forked_from_thread、
/// conversation、turn、parent_turn、root_turn、window、context_window 及其在头/正文里
/// 出现过的连字符与 camelCase 写法。
pub const IDENTITY_ID_KEYS: &[&str] = &[
    "session_id",
    "session-id",
    "sessionId",
    "thread_id",
    "thread-id",
    "threadId",
    "parent_thread_id",
    "parent-thread-id",
    "parentThreadId",
    "root_thread_id",
    "root-thread-id",
    "rootThreadId",
    "forked_from_thread_id",
    "forked-from-thread-id",
    "forkedFromThreadId",
    "conversation_id",
    "conversation-id",
    "conversationId",
    "turn_id",
    "turn-id",
    "turnId",
    "parent_turn_id",
    "parent-turn-id",
    "parentTurnId",
    "root_turn_id",
    "root-turn-id",
    "rootTurnId",
    "window_id",
    "window-id",
    "windowId",
    "x-codex-window-id",
    "context_window_id",
    "context-window-id",
    "contextWindowId",
    "x-client-request-id",
];

/// 正文里作为整体身份承载于 `client_metadata` 的缓存键；官方客户端用它携带会话身份。
const PROMPT_CACHE_KEY: &str = "prompt_cache_key";

/// 只处理工作区根（对象键）与远端 URL 值；标签、提交号等原样保留。
const WORKSPACES_KEY: &str = "workspaces";
const ASSOCIATED_REMOTE_URLS_KEY: &str = "associated_remote_urls";

/// 账号内稳定的身份伪名。
#[derive(Clone)]
pub struct IdentityPseudonym {
    seed: [u8; 32],
}

impl IdentityPseudonym {
    /// 以账号的安装身份为种子；安装身份只由当前账号写入，天然是账号维度。
    #[must_use]
    pub fn for_account(installation_id: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"cpr-identity-pseudonym-v1\0");
        hasher.update(installation_id.as_bytes());
        let mut seed = [0_u8; 32];
        seed.copy_from_slice(&hasher.finalize());
        Self { seed }
    }

    /// 原值 → 伪名。
    ///
    /// 输出固定为 36 位 UUID：这些字段在上游都是不透明 id，UUID 形状对会话、线程、
    /// 窗口与工作区根都合法，也保持 turn metadata 的 ASCII 编码合同。已经是空串的值
    /// 不伪名化——空值不是身份。
    #[must_use]
    pub fn of(&self, value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.seed);
        hasher.update([0_u8]);
        hasher.update(value.as_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        // 置成 RFC 4122 v4 形状，避免上游按 UUID 解析时拒绝。
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let hex = hex::encode(bytes);
        format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        )
    }
}

/// 伪名化一层 JSON object 里的身份键与缓存键；返回是否有改写。
///
/// 只替换已存在且为非空字符串的值，不新增键、不动其他字段。
pub fn pseudonymize_identity_fields(
    object: &mut Map<String, Value>,
    pseudonym: &IdentityPseudonym,
) -> bool {
    let mut changed = false;
    for key in IDENTITY_ID_KEYS.iter().chain([&PROMPT_CACHE_KEY]) {
        let Some(value) = object.get_mut(*key) else {
            continue;
        };
        let Some(current) = value.as_str() else {
            continue;
        };
        if current.is_empty() {
            continue;
        }
        *value = Value::String(pseudonym.of(current));
        changed = true;
    }
    changed
}

/// 伪名化 `workspaces`：对象键是工作区根，`associated_remote_urls` 的值是远端仓库地址；
/// 其余字段（`label`、提交号等）逐字保留。返回是否有改写。
pub fn pseudonymize_workspaces(
    object: &mut Map<String, Value>,
    pseudonym: &IdentityPseudonym,
) -> bool {
    let Some(Value::Object(workspaces)) = object.get_mut(WORKSPACES_KEY) else {
        return false;
    };
    if workspaces.is_empty() {
        return false;
    }
    let original = std::mem::take(workspaces);
    let mut rebuilt = Map::with_capacity(original.len());
    for (root, mut workspace) in original {
        if let Some(workspace) = workspace.as_object_mut()
            && let Some(Value::Array(urls)) = workspace.get_mut(ASSOCIATED_REMOTE_URLS_KEY)
        {
            for url in urls.iter_mut() {
                if let Some(value) = url.as_str().filter(|value| !value.is_empty()) {
                    *url = Value::String(pseudonym.of(value));
                }
            }
        }
        rebuilt.insert(pseudonym.of(&root), workspace);
    }
    *workspaces = rebuilt;
    true
}
