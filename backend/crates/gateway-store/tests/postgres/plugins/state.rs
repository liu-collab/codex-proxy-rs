//! 验证插件私有状态迁移的顺序、配额与代次资源回收

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use gateway_admin::{
    model::{
        Revision,
        plugins::{
            PluginSource,
            instances::PluginInstance,
            state::{
                ApplyPluginStateMigration, DeletePluginState, PluginStateCommit,
                PluginStateConfiguration, PluginStateMigrationAction, PluginStateMigrationChange,
                PluginStateOwnerRequest, PluginStateSchema, PutPluginState,
            },
        },
    },
    ports::{
        plugins::{PluginStateStore, PluginStateStoreErrorKind, PluginStore},
        store::AdminStoreErrorKind,
    },
};
use gateway_store::postgres::PgPluginStore;
use serde_json::json;

use super::{
    super::TestDatabase,
    artifacts::{artifact, context, initialize_revision},
};

fn schema(version: u32, migrates_from: Vec<u32>, maximum_records: u32) -> PluginStateSchema {
    PluginStateSchema {
        namespace: "cache".into(),
        schema_version: version,
        schema_sha256: char::from_digit(version, 16)
            .unwrap()
            .to_string()
            .repeat(64),
        schema: json!({
            "type": "object",
            "required": ["value"],
            "properties": {"value": {"type": "integer"}},
            "additionalProperties": false,
        }),
        maximum_records,
        maximum_bytes: 1024,
        maximum_value_bytes: 128,
        migrates_from,
    }
}

fn configuration(schema: PluginStateSchema) -> PluginStateConfiguration {
    PluginStateConfiguration {
        namespaces: vec![schema],
    }
}

fn instance(artifact_sha256: String) -> PluginInstance {
    PluginInstance {
        id: uuid::Uuid::now_v7().to_string(),
        name: "State fixture".into(),
        artifact_sha256,
        enabled: false,
        trusted_process: true,
        configuration: json!({}),
        secrets: BTreeMap::new(),

        bindings: vec![],
        revision: Revision::new(1).unwrap(),
    }
}

fn owner_request(
    instance: &PluginInstance,
    configuration: PluginStateConfiguration,
) -> PluginStateOwnerRequest {
    PluginStateOwnerRequest {
        instance_id: instance.id.clone(),
        artifact_sha256: instance.artifact_sha256.clone(),
        instance_revision: instance.revision,
        configuration,
    }
}

async fn install_version(
    store: &PgPluginStore,
    digest: char,
    version: &str,
) -> gateway_admin::model::plugins::PluginArtifactMutation {
    let mut artifact = artifact(digest, &["linux-x86_64"]);
    artifact.metadata.version = version.into();
    let installed = store
        .install_artifact(artifact, PluginSource::Upload, &context())
        .await
        .unwrap();
    store
        .accept_artifact(&installed.artifact.metadata.sha256, &context())
        .await
        .unwrap()
}

async fn migrate_all(
    store: &PgPluginStore,
    transition_id: &str,
    namespace: &str,
    batch_size: u32,
) -> Vec<String> {
    let mut keys = Vec::new();
    loop {
        let batch = store
            .migration_batch(transition_id, namespace, batch_size)
            .await
            .unwrap();
        let complete = batch.records.is_empty();
        let expected_keys: Vec<_> = batch
            .records
            .iter()
            .map(|record| record.key.clone())
            .collect();
        keys.extend(expected_keys.iter().cloned());
        store
            .apply_migration_batch(ApplyPluginStateMigration {
                transition_id: transition_id.to_owned(),
                namespace: namespace.to_owned(),
                cursor: batch.cursor,
                changes: expected_keys
                    .iter()
                    .map(|key| PluginStateMigrationChange {
                        key: key.clone(),
                        action: PluginStateMigrationAction::Keep,
                    })
                    .collect(),
                expected_keys,
            })
            .await
            .unwrap();
        if complete {
            return keys;
        }
    }
}

async fn transition_fixture(
    store: &PgPluginStore,
) -> (PluginInstance, String, PluginStateConfiguration) {
    let installed = install_version(store, 'a', "1.0.0").await;
    let saved = store
        .save_instance_with_state(
            instance(installed.artifact.metadata.sha256),
            installed.config_revision,
            PluginStateCommit {
                configuration: configuration(schema(1, vec![], 10)),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let target = install_version(store, 'b', "2.0.0").await;
    (
        saved.instance,
        target.artifact.metadata.sha256,
        configuration(schema(2, vec![1], 10)),
    )
}

#[tokio::test]
async fn cancelled_migration_releases_ownership_and_can_be_retried() {
    let Some(database) = TestDatabase::create("plugin_state_cancel").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = Arc::new(PgPluginStore::new(database.pool.clone()));
    let (instance, digest, target) = transition_fixture(&store).await;
    let (started, migration) = tokio::sync::oneshot::channel();
    let task = {
        let store = store.clone();
        let instance = instance.clone();
        let digest = digest.clone();
        let target = target.clone();
        tokio::spawn(async move {
            let transition = store
                .begin_transition(&instance.id, instance.revision, &digest, target)
                .await
                .unwrap();
            started.send(transition.id.clone()).unwrap();
            std::future::pending::<()>().await;
            drop(transition);
        })
    };
    let abandoned_id = migration.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    // 关闭会话由 PostgreSQL 异步处理，重试只等待所有权释放，不调用清理接口补偿
    let retried = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match store
                .begin_transition(&instance.id, instance.revision, &digest, target.clone())
                .await
            {
                Ok(transition) => break transition,
                Err(error) if error.kind() == PluginStateStoreErrorKind::Conflict => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(error) => panic!("unexpected retry failure: {error:?}"),
            }
        }
    })
    .await
    .expect("cancelled migration must release its session lock");
    assert_ne!(retried.id, abandoned_id);
    let retained: bool = sqlx::query_scalar(
        "select exists(select 1 from plugin_state_generations where transition_id=$1)",
    )
    .bind(uuid::Uuid::parse_str(&abandoned_id).unwrap())
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert!(!retained);
    let current = store.load_instances().await.unwrap();
    assert_eq!(current.instances[0].revision, instance.revision);
    assert!(!current.instances[0].enabled);
    drop(retried);
    database.close().await;
}

#[tokio::test]
async fn startup_recovery_preserves_live_owners_and_does_not_starve_a_single_connection_pool() {
    let Some(database) = TestDatabase::create("plugin_state_recovery").await else {
        return;
    };
    initialize_revision(&database).await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database.pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let store = PgPluginStore::new(pool.clone());
    let (instance, digest, target) = transition_fixture(&store).await;
    let transition = tokio::time::timeout(
        Duration::from_secs(3),
        store.begin_transition(&instance.id, instance.revision, &digest, target.clone()),
    )
    .await
    .expect("migration must not retain a pooled connection")
    .unwrap();
    let reopened = PgPluginStore::new(pool.clone());
    assert_eq!(
        reopened
            .recover_abandoned_state_transitions()
            .await
            .unwrap(),
        0
    );
    let error = reopened
        .begin_transition(&instance.id, instance.revision, &digest, target)
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), PluginStateStoreErrorKind::Conflict);
    let retained = transition.clone();
    drop(transition);
    assert_eq!(
        reopened
            .recover_abandoned_state_transitions()
            .await
            .unwrap(),
        0
    );
    drop(retained);
    tokio::time::timeout(Duration::from_secs(3), async {
        while reopened
            .recover_abandoned_state_transitions()
            .await
            .unwrap()
            == 0
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("startup recovery must discard ownerless staging");
    let current = reopened.load_instances().await.unwrap();
    assert_eq!(current.instances[0].revision, instance.revision);
    assert!(!current.instances[0].enabled);
    pool.close().await;
    database.close().await;
}

#[tokio::test]
async fn lost_migration_session_cannot_promote_after_another_owner_recovers() {
    let Some(database) = TestDatabase::create("plugin_state_crash").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let (instance, digest, target) = transition_fixture(&store).await;
    let transition = store
        .begin_transition(&instance.id, instance.revision, &digest, target.clone())
        .await
        .unwrap();
    migrate_all(&store, &transition.id, "cache", 100).await;
    // 直接中断持锁数据库会话，覆盖宿主崩溃后的服务端锁释放，无需执行 Rust 清理逻辑
    let terminated: Vec<bool> = sqlx::query_scalar(
        "select pg_terminate_backend(pid) from pg_locks where locktype='advisory' and granted \
         and classid::bigint=((hashtextextended($1,739220)>>32)&4294967295) \
         and objid::bigint=(hashtextextended($1,739220)&4294967295) and objsubid=1",
    )
    .bind(&instance.id)
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(terminated, vec![true]);
    let reopened = PgPluginStore::new(database.pool.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while reopened
            .recover_abandoned_state_transitions()
            .await
            .unwrap()
            == 0
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("recovery must reclaim the terminated session's staging");
    assert!(
        store
            .migration_batch(&transition.id, "cache", 100)
            .await
            .is_err()
    );
    let retried = reopened
        .begin_transition(&instance.id, instance.revision, &digest, target.clone())
        .await
        .unwrap();
    assert_ne!(retried.id, transition.id);
    let current = store.load_instances().await.unwrap();
    let mut upgraded = instance.clone();
    upgraded.artifact_sha256 = digest;
    // 旧任务即使已完成全部批次，接管后也不能晋升或覆盖新迁移
    assert_eq!(
        store
            .save_instance_with_state(
                upgraded,
                current.config_revision,
                PluginStateCommit {
                    configuration: target,
                    transition_id: Some(transition.id.clone()),
                },
                &context(),
            )
            .await
            .err()
            .unwrap()
            .kind(),
        AdminStoreErrorKind::Conflict
    );
    let unchanged = store.load_instances().await.unwrap();
    assert_eq!(unchanged.config_revision, current.config_revision);
    assert_eq!(unchanged.instances[0].revision, instance.revision);
    assert_eq!(
        unchanged.instances[0].artifact_sha256,
        instance.artifact_sha256
    );
    assert!(!unchanged.instances[0].enabled);
    // 迟到的旧迁移清理不能删除新所有者的暂存代次
    store.abort_transition(&transition.id).await.unwrap();
    assert!(
        reopened
            .migration_batch(&retried.id, "cache", 100)
            .await
            .is_ok()
    );
    drop(retried);
    drop(transition);
    database.close().await;
}

#[tokio::test]
async fn migration_preserves_byte_order_for_mixed_case_and_unicode_keys() {
    let Some(database) = TestDatabase::create("plugin_state_key_order").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let state = configuration(schema(1, vec![], 10));
    let saved = store
        .save_instance_with_state(
            instance(installed.artifact.metadata.sha256),
            installed.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&saved.instance, state))
        .await
        .unwrap()
        .unwrap();
    for key in ["a", "B", "é", "Z", "中", "a10", "a2"] {
        store
            .put(
                &owner,
                PutPluginState {
                    namespace: "cache".into(),
                    key: key.into(),
                    value: json!({"value": 1}),
                    expected_version: None,
                },
            )
            .await
            .unwrap();
    }
    let target = install_version(&store, 'b', "2.0.0").await;
    let transition = store
        .begin_transition(
            &saved.instance.id,
            saved.instance.revision,
            &target.artifact.metadata.sha256,
            configuration(schema(2, vec![1], 10)),
        )
        .await
        .unwrap();
    assert_eq!(
        migrate_all(&store, &transition.id, "cache", 3).await,
        ["B", "Z", "a", "a10", "a2", "é", "中"]
    );
    database.close().await;
}

#[tokio::test]
async fn completed_migration_releases_old_records_and_artifact() {
    let Some(database) = TestDatabase::create("plugin_state_release").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let old_digest = installed.artifact.metadata.sha256;
    let state = configuration(schema(1, vec![], 2));
    let saved = store
        .save_instance_with_state(
            instance(old_digest.clone()),
            installed.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&saved.instance, state))
        .await
        .unwrap()
        .unwrap();
    store
        .put(
            &owner,
            PutPluginState {
                namespace: "cache".into(),
                key: "retained".into(),
                value: json!({"value": 1}),
                expected_version: None,
            },
        )
        .await
        .unwrap();
    let target = install_version(&store, 'b', "2.0.0").await;
    let state = configuration(schema(2, vec![1], 2));
    let transition = store
        .begin_transition(
            &saved.instance.id,
            saved.instance.revision,
            &target.artifact.metadata.sha256,
            state.clone(),
        )
        .await
        .unwrap();
    migrate_all(&store, &transition.id, "cache", 100).await;
    let mut upgraded = saved.instance;
    upgraded.artifact_sha256 = target.artifact.metadata.sha256;
    let upgraded = store
        .save_instance_with_state(
            upgraded,
            target.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: Some(transition.id),
            },
            &context(),
        )
        .await
        .unwrap();
    store
        .delete_artifact(&old_digest, &context())
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&upgraded.instance, state))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .get(&owner, "cache", "retained")
            .await
            .unwrap()
            .unwrap()
            .value,
        json!({"value": 1})
    );
    let counts: (i64, i64) = sqlx::query_as(
        "select (select count(*) from plugin_state_generations), \
                (select count(*) from plugin_state_records)",
    )
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(counts, (1, 1));
    database.close().await;
}

#[tokio::test]
async fn schema_budget_uses_compact_json_not_database_formatting() {
    let Some(database) = TestDatabase::create("plugin_state_schema_budget").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let mut namespace = schema(1, vec![], 2);
    namespace.schema = json!({
        "type": "string",
        "enum": (0..4000).map(|index| format!("v{index:04}")).collect::<Vec<_>>(),
    });
    assert!(serde_json::to_vec(&namespace.schema).unwrap().len() <= 32 * 1024);
    let database_bytes: i32 = sqlx::query_scalar("select octet_length($1::jsonb::text)")
        .bind(&namespace.schema)
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert!(database_bytes > 32 * 1024);
    let state = configuration(namespace);
    let saved = store
        .save_instance_with_state(
            instance(installed.artifact.metadata.sha256),
            installed.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    assert!(
        store
            .load_owner(owner_request(&saved.instance, state))
            .await
            .unwrap()
            .is_some()
    );
    database.close().await;
}

#[tokio::test]
async fn failed_multi_namespace_promotion_restores_cleaned_source_generations() {
    let Some(database) = TestDatabase::create("plugin_state_promotion_rollback").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let old_digest = installed.artifact.metadata.sha256;
    let mut state = configuration(schema(1, vec![], 2));
    let mut second = state.namespaces[0].clone();
    second.namespace = "settings".into();
    state.namespaces.push(second);
    let saved = store
        .save_instance_with_state(
            instance(old_digest.clone()),
            installed.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&saved.instance, state))
        .await
        .unwrap()
        .unwrap();
    for namespace in ["cache", "settings"] {
        store
            .put(
                &owner,
                PutPluginState {
                    namespace: namespace.into(),
                    key: "retained".into(),
                    value: json!({"value": 1}),
                    expected_version: None,
                },
            )
            .await
            .unwrap();
    }
    let target = install_version(&store, 'b', "2.0.0").await;
    let mut state = configuration(schema(2, vec![1], 2));
    let mut second = state.namespaces[0].clone();
    second.namespace = "settings".into();
    state.namespaces.push(second);
    let transition = store
        .begin_transition(
            &saved.instance.id,
            saved.instance.revision,
            &target.artifact.metadata.sha256,
            state.clone(),
        )
        .await
        .unwrap();
    migrate_all(&store, &transition.id, "cache", 100).await;
    let mut upgraded = saved.instance;
    upgraded.artifact_sha256 = target.artifact.metadata.sha256;
    let commit = PluginStateCommit {
        configuration: state.clone(),
        transition_id: Some(transition.id.clone()),
    };
    let error = store
        .save_instance_with_state(
            upgraded.clone(),
            target.config_revision,
            commit.clone(),
            &context(),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), AdminStoreErrorKind::Conflict);
    // cache 已走过清理步骤，settings 未完成导致整笔事务回滚，旧身份和两份数据仍有效
    for namespace in ["cache", "settings"] {
        assert_eq!(
            store
                .get(&owner, namespace, "retained")
                .await
                .unwrap()
                .unwrap()
                .value,
            json!({"value": 1})
        );
    }
    migrate_all(&store, &transition.id, "settings", 100).await;
    let upgraded = store
        .save_instance_with_state(upgraded, target.config_revision, commit, &context())
        .await
        .unwrap();
    store
        .delete_artifact(&old_digest, &context())
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&upgraded.instance, state))
        .await
        .unwrap()
        .unwrap();
    for namespace in ["cache", "settings"] {
        assert_eq!(
            store
                .get(&owner, namespace, "retained")
                .await
                .unwrap()
                .unwrap()
                .value,
            json!({"value": 1})
        );
    }
    database.close().await;
}

#[tokio::test]
async fn removing_empty_namespace_releases_its_artifact_reference() {
    let Some(database) = TestDatabase::create("plugin_state_remove_namespace").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let old_digest = installed.artifact.metadata.sha256;
    let saved = store
        .save_instance_with_state(
            instance(old_digest.clone()),
            installed.config_revision,
            PluginStateCommit {
                configuration: configuration(schema(1, vec![], 2)),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let target = install_version(&store, 'b', "2.0.0").await;
    let mut upgraded = saved.instance;
    upgraded.artifact_sha256 = target.artifact.metadata.sha256;
    store
        .save_instance_with_state(
            upgraded,
            target.config_revision,
            PluginStateCommit {
                configuration: PluginStateConfiguration { namespaces: vec![] },
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    store
        .delete_artifact(&old_digest, &context())
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar("select count(*) from plugin_state_generations")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    database.close().await;
}

#[tokio::test]
async fn state_cas_quota_and_fence_are_independent_from_global_revision() {
    let Some(database) = TestDatabase::create("plugin_state_cas").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let installed = install_version(&store, 'a', "1.0.0").await;
    let state = configuration(schema(1, vec![], 2));
    let saved = store
        .save_instance_with_state(
            instance(installed.artifact.metadata.sha256),
            installed.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let owner = store
        .load_owner(owner_request(&saved.instance, state.clone()))
        .await
        .unwrap()
        .unwrap();

    // 无关制品安装会推进全局配置 revision，但不能撤销实例自己的状态 fence
    let unrelated = install_version(&store, 'b', "2.0.0").await;
    let first = store
        .put(
            &owner,
            PutPluginState {
                namespace: "cache".into(),
                key: "a".into(),
                value: json!({"value": 1}),
                expected_version: None,
            },
        )
        .await
        .unwrap();
    let second = store
        .put(
            &owner,
            PutPluginState {
                namespace: "cache".into(),
                key: "b".into(),
                value: json!({"value": 2}),
                expected_version: None,
            },
        )
        .await
        .unwrap();
    assert_eq!((first.version, second.version), (1, 2));
    assert_eq!(
        store
            .put(
                &owner,
                PutPluginState {
                    namespace: "cache".into(),
                    key: "c".into(),
                    value: json!({"value": 3}),
                    expected_version: None,
                },
            )
            .await
            .unwrap_err()
            .kind(),
        PluginStateStoreErrorKind::Quota
    );
    let updated = store
        .put(
            &owner,
            PutPluginState {
                namespace: "cache".into(),
                key: "a".into(),
                value: json!({"value": 4}),
                expected_version: Some(first.version),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.version, 3);
    assert_eq!(
        store
            .delete(
                &owner,
                DeletePluginState {
                    namespace: "cache".into(),
                    key: "a".into(),
                    expected_version: first.version,
                },
            )
            .await
            .unwrap_err()
            .kind(),
        PluginStateStoreErrorKind::Conflict
    );
    assert!(
        store
            .delete(
                &owner,
                DeletePluginState {
                    namespace: "cache".into(),
                    key: "a".into(),
                    expected_version: updated.version,
                },
            )
            .await
            .unwrap()
    );
    let recreated = store
        .put(
            &owner,
            PutPluginState {
                namespace: "cache".into(),
                key: "a".into(),
                value: json!({"value": 5}),
                expected_version: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(recreated.version, 4, "删除后不能产生可 ABA 的记录版本");
    assert_eq!(
        store.load_instances().await.unwrap().config_revision,
        unrelated.config_revision,
        "普通状态写入不能推进全局配置 revision"
    );
    let mut undersized = state.clone();
    undersized.namespaces[0].maximum_value_bytes = 4;
    assert_eq!(
        store
            .transition_required(&saved.instance.id, &undersized)
            .await
            .unwrap_err()
            .kind(),
        PluginStateStoreErrorKind::Conflict,
        "配额收紧不能让已有记录超过单值上限"
    );

    // 同一实例的配置提交会轮换 fence；旧进程即使仍持有 owner 也不能继续写
    let rebound = store
        .save_instance_with_state(
            saved.instance.clone(),
            unrelated.config_revision,
            PluginStateCommit {
                configuration: state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let error = store.get(&owner, "cache", "a").await.err().unwrap();
    assert_eq!(error.kind(), PluginStateStoreErrorKind::PermissionDenied);
    let rebound_owner = store
        .load_owner(owner_request(&rebound.instance, state))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .get(&rebound_owner, "cache", "a")
            .await
            .unwrap()
            .unwrap()
            .value,
        json!({"value": 5})
    );
    database.close().await;
}

#[tokio::test]
async fn migration_promotes_atomically_and_incomplete_staging_cannot_replace_active_state() {
    let Some(database) = TestDatabase::create("plugin_state_migration").await else {
        return;
    };
    initialize_revision(&database).await;
    let store = PgPluginStore::new(database.pool.clone());
    let first_artifact = install_version(&store, 'c', "1.0.0").await;
    let second_artifact = install_version(&store, 'd', "2.0.0").await;
    let third_artifact = install_version(&store, 'e', "3.0.0").await;
    let first_state = configuration(schema(1, vec![], 4));
    let saved = store
        .save_instance_with_state(
            instance(first_artifact.artifact.metadata.sha256),
            third_artifact.config_revision,
            PluginStateCommit {
                configuration: first_state.clone(),
                transition_id: None,
            },
            &context(),
        )
        .await
        .unwrap();
    let first_owner = store
        .load_owner(owner_request(&saved.instance, first_state))
        .await
        .unwrap()
        .unwrap();
    for (key, value) in [("a", 1), ("b", 2)] {
        store
            .put(
                &first_owner,
                PutPluginState {
                    namespace: "cache".into(),
                    key: key.into(),
                    value: json!({"value": value}),
                    expected_version: None,
                },
            )
            .await
            .unwrap();
    }

    let second_state = configuration(schema(2, vec![1], 4));
    assert!(
        store
            .transition_required(&saved.instance.id, &second_state)
            .await
            .unwrap()
    );
    let transition = store
        .begin_transition(
            &saved.instance.id,
            saved.instance.revision,
            &second_artifact.artifact.metadata.sha256,
            second_state.clone(),
        )
        .await
        .unwrap();
    let batch = store
        .migration_batch(&transition.id, "cache", 64)
        .await
        .unwrap();
    assert_eq!(
        batch
            .records
            .iter()
            .map(|record| record.key.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    store
        .apply_migration_batch(ApplyPluginStateMigration {
            transition_id: transition.id.clone(),
            namespace: "cache".into(),
            cursor: batch.cursor,
            expected_keys: vec!["a".into(), "b".into()],
            changes: vec![
                PluginStateMigrationChange {
                    key: "a".into(),
                    action: PluginStateMigrationAction::Keep,
                },
                PluginStateMigrationChange {
                    key: "b".into(),
                    action: PluginStateMigrationAction::Replace(json!({"value": 20})),
                },
            ],
        })
        .await
        .unwrap();
    let terminal = store
        .migration_batch(&transition.id, "cache", 64)
        .await
        .unwrap();
    assert!(terminal.records.is_empty());
    store
        .apply_migration_batch(ApplyPluginStateMigration {
            transition_id: transition.id.clone(),
            namespace: "cache".into(),
            cursor: terminal.cursor,
            expected_keys: vec![],
            changes: vec![],
        })
        .await
        .unwrap();
    assert_eq!(
        store.load_instances().await.unwrap().config_revision,
        saved.config_revision,
        "状态迁移批次不能推进全局配置 revision"
    );

    let mut upgraded = saved.instance.clone();
    upgraded.artifact_sha256 = second_artifact.artifact.metadata.sha256;
    let upgraded = store
        .save_instance_with_state(
            upgraded,
            saved.config_revision,
            PluginStateCommit {
                configuration: second_state.clone(),
                transition_id: Some(transition.id.clone()),
            },
            &context(),
        )
        .await
        .unwrap();
    let error = store.get(&first_owner, "cache", "a").await.err().unwrap();
    assert_eq!(error.kind(), PluginStateStoreErrorKind::PermissionDenied);
    let second_owner = store
        .load_owner(owner_request(&upgraded.instance, second_state))
        .await
        .unwrap()
        .unwrap();
    let migrated = store
        .get(&second_owner, "cache", "b")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrated.schema_version, 2);
    assert_eq!(migrated.value, json!({"value": 20}));
    drop(transition);

    let third_state = configuration(schema(3, vec![2], 4));
    let incomplete = store
        .begin_transition(
            &upgraded.instance.id,
            upgraded.instance.revision,
            &third_artifact.artifact.metadata.sha256,
            third_state.clone(),
        )
        .await
        .unwrap();
    let mut rejected = upgraded.instance.clone();
    rejected.artifact_sha256 = third_artifact.artifact.metadata.sha256;
    let error = store
        .save_instance_with_state(
            rejected,
            upgraded.config_revision,
            PluginStateCommit {
                configuration: third_state,
                transition_id: Some(incomplete.id.clone()),
            },
            &context(),
        )
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind(), AdminStoreErrorKind::Conflict);
    let current = store.load_instances().await.unwrap();
    assert_eq!(current.config_revision, upgraded.config_revision);
    assert_eq!(
        current.instances[0].artifact_sha256,
        upgraded.instance.artifact_sha256
    );
    store.abort_transition(&incomplete.id).await.unwrap();
    assert_eq!(
        store
            .get(&second_owner, "cache", "b")
            .await
            .unwrap()
            .unwrap()
            .value,
        json!({"value": 20})
    );
    database.close().await;
}
