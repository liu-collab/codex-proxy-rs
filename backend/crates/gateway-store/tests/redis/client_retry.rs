//! 验证失败轮次标记的隔离、过期与并发条件清理

use std::time::Duration;

use gateway_core::{
    engine::{
        ModelRequestId,
        client_retry::{ClientRetryKey, ClientRetryStore},
    },
    operation::ClientTurn,
    policy::ClientApiKeyId,
};
use gateway_store::redis::RedisClientRetryRepository;
use redis::aio::ConnectionManager;
use uuid::Uuid;

async fn repository() -> Option<(RedisClientRetryRepository, ConnectionManager, String)> {
    let url = crate::support::test_env("CPR_TEST_REDIS_URL")?;
    let client = redis::Client::open(url).unwrap();
    let connection = client.get_connection_manager().await.unwrap();
    let namespace = format!("client-retry-test-{}", Uuid::new_v4());
    Some((
        RedisClientRetryRepository::new(connection.clone(), &namespace).unwrap(),
        connection,
        namespace,
    ))
}

fn key(client: &str, thread: &str, turn: &str) -> ClientRetryKey {
    ClientRetryKey {
        client_key_id: ClientApiKeyId::new(client).unwrap(),
        turn: ClientTurn::new(thread.to_owned(), turn.to_owned()).unwrap(),
    }
}

#[tokio::test]
async fn client_retry_versions_should_protect_newer_failures() {
    let Some((store, _, _)) = repository().await else {
        return;
    };
    let key = key("key_one", "thread", "turn");
    let old = ModelRequestId::new("req_old").unwrap();
    let newer = ModelRequestId::new("req_newer").unwrap();
    store
        .record_failure(&key, &old, Duration::from_secs(30))
        .await
        .unwrap();
    store
        .record_failure(&key, &newer, Duration::from_secs(30))
        .await
        .unwrap();
    store.clear_failure(&key, &old).await.unwrap();
    assert_eq!(store.load(&key).await.unwrap(), Some(newer.clone()));
    store.clear_failure(&key, &newer).await.unwrap();
    assert_eq!(store.load(&key).await.unwrap(), None);
}

#[tokio::test]
async fn client_retry_should_isolate_all_identity_parts() {
    let Some((store, _, _)) = repository().await else {
        return;
    };
    let first = key("key_one", "ab", "c");
    let version = ModelRequestId::new("req_first").unwrap();
    store
        .record_failure(&first, &version, Duration::from_secs(30))
        .await
        .unwrap();
    for other in [
        key("key_other", "ab", "c"),
        key("key_one", "a", "bc"),
        key("key_one", "ab", "other"),
    ] {
        assert_eq!(store.load(&other).await.unwrap(), None);
    }
    assert_eq!(store.load(&first).await.unwrap(), Some(version.clone()));
    store.clear_failure(&first, &version).await.unwrap();
}

#[tokio::test]
async fn client_retry_should_refresh_ttl_on_failure_and_expire_without_cleanup_worker() {
    let Some((store, mut connection, namespace)) = repository().await else {
        return;
    };
    let key = key("key_one", "thread", "turn");
    let old = ModelRequestId::new("req_old").unwrap();
    let newer = ModelRequestId::new("req_newer").unwrap();
    store
        .record_failure(&key, &old, Duration::from_secs(30))
        .await
        .unwrap();
    store
        .record_failure(&key, &newer, Duration::from_secs(300))
        .await
        .unwrap();
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{namespace}:*"))
        .query_async(&mut connection)
        .await
        .unwrap();
    assert_eq!(keys.len(), 1);
    let ttl: i64 = redis::cmd("PTTL")
        .arg(&keys[0])
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!(ttl > 290_000 && ttl <= 300_000);
    store
        .record_failure(&key, &newer, Duration::from_millis(100))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while store.load(&key).await.unwrap().is_some() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Redis expires marker");
}
