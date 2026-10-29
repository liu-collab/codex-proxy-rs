//! 验证 Store 认证、凭据编码与只读检查 Bundle 的写入边界

use gateway_core::{
    account::{
        CredentialRevision, NewProviderAccount, PlaintextCredential, ProviderAccount,
        ProviderAccountId,
    },
    routing::ProviderKind,
};
use gateway_store::StoreConfig;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::support::config_connection;

#[tokio::test]
async fn inspection_bundle_does_not_migrate_and_rejects_business_writes() {
    let (Some(database), Some(redis)) = (
        crate::support::test_env("CPR_TEST_DATABASE_URL"),
        crate::support::test_env("CPR_TEST_REDIS_URL"),
    ) else {
        return;
    };
    let schema = format!("cpr_inspection_{}", uuid::Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database)
        .await
        .unwrap();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("create schema {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let mut database = url::Url::parse(&database).unwrap();
    database
        .query_pairs_mut()
        .append_pair("options", &format!("-csearch_path={schema}"));
    let mut config: StoreConfig = serde_json::from_value(json!({
        "database":config_connection(database), "redis":config_connection(url::Url::parse(&redis).unwrap()),
    })).unwrap();
    let directory = tempfile::tempdir().unwrap();
    config.resolve_and_validate(directory.path()).unwrap();
    let inspection = gateway_store::initialize_read_only(config.clone())
        .await
        .unwrap();
    assert!(
        inspection
            .admin_ports()
            .plugins()
            .load_instances()
            .await
            .is_err()
    );
    let tables: i64 =
        sqlx::query_scalar("select count(*) from information_schema.tables where table_schema=$1")
            .bind(&schema)
            .fetch_one(&admin)
            .await
            .unwrap();
    assert_eq!(tables, 0, "只读帮助不能在空库创建迁移表或业务表");
    drop(inspection);
    let writable = gateway_store::initialize(config.clone()).await.unwrap();
    let inspection = gateway_store::initialize_read_only(config).await.unwrap();
    let create = |id: &str| NewProviderAccount {
        account: ProviderAccount::new(
            ProviderAccountId::new(id.to_owned()).unwrap(),
            ProviderKind::new("example").unwrap(),
            "inspection test".into(),
            None,
            "api_key".into(),
            CredentialRevision::new(1).unwrap(),
            None,
        ),
        credential: PlaintextCredential::new(
            json!({"key":"test-only"}).as_object().unwrap().clone(),
        ),
        model_access: None,
    };
    writable
        .provider_ports()
        .accounts()
        .create_account(create("acct_writable"))
        .await
        .unwrap();
    assert!(
        inspection
            .provider_ports()
            .accounts()
            .create_account(create("acct_readonly"))
            .await
            .is_err()
    );
    let accounts = inspection
        .provider_ports()
        .accounts()
        .list_accounts()
        .await
        .unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id().as_str(), "acct_writable");
    drop(inspection);
    drop(writable);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("drop schema {schema} cascade")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn store_bundle_authenticates_configured_and_redis_acl_credentials() {
    let (Some(database), Some(redis_url)) = (
        crate::support::test_env("CPR_TEST_DATABASE_URL"),
        crate::support::test_env("CPR_TEST_REDIS_URL"),
    ) else {
        return;
    };
    let document = json!({
        "database": config_connection(url::Url::parse(&database).unwrap()),
        "redis": config_connection(url::Url::parse(&redis_url).unwrap()),
    });
    let mut admin = redis::Client::open(redis_url.as_str())
        .unwrap()
        .get_connection_manager()
        .await
        .unwrap();
    let username = format!("cpr_password_{}", uuid::Uuid::new_v4().simple());
    let password = "Managed%41%25%FF@:/?#%&+$'\"\\ secret9!";
    redis::cmd("ACL")
        .arg("SETUSER")
        .arg(&username)
        .arg("on")
        .arg(format!(">{password}"))
        .arg("~*")
        .arg("&*")
        .arg("+@all")
        .query_async::<()>(&mut admin)
        .await
        .unwrap();
    let mut acl_url = url::Url::parse(&redis_url).unwrap();
    acl_url.set_username(&username).unwrap();
    acl_url.set_password(None).unwrap();
    let mut acl_document = document.clone();
    acl_document["redis"] = json!({"url": acl_url.as_str(), "password": password});
    let directory = tempfile::tempdir().unwrap();
    let mut results = Vec::new();
    // 同时走 Store 的配置解析、URL 构造与真实连接路径，断言前先清理临时 ACL 用户
    for document in [document, acl_document] {
        let mut config: StoreConfig = serde_json::from_value(document).unwrap();
        let result = async {
            config.resolve_and_validate(directory.path())?;
            let bundle = gateway_store::initialize_read_only(config).await?;
            let mut states = Vec::new();
            for probe in bundle.health_probes() {
                states.push(probe.check().await);
            }
            Ok::<_, gateway_store::StoreError>(states)
        }
        .await;
        results.push(result);
    }
    redis::cmd("ACL")
        .arg("DELUSER")
        .arg(&username)
        .query_async::<i64>(&mut admin)
        .await
        .unwrap();
    for result in results {
        let states = result.expect("Store credentials authenticate without double encoding");
        assert!(!states.is_empty());
        assert!(
            states
                .iter()
                .all(|state| *state == gateway_core::health::HealthState::Healthy)
        );
    }
}

#[tokio::test]
async fn store_bundle_authenticates_postgres_password_with_percent_sequences() {
    let (Some(database), Some(redis_url)) = (
        crate::support::test_env("CPR_TEST_DATABASE_URL"),
        crate::support::test_env("CPR_TEST_REDIS_URL"),
    ) else {
        return;
    };
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database)
        .await
        .unwrap();
    let username = format!("cpr_password_{}", uuid::Uuid::new_v4().simple());
    let password = "Managed%41%25%FFPassword9!";
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE {username} LOGIN PASSWORD '{password}'"
    )))
    .execute(&admin)
    .await
    .unwrap();
    let mut database_url = url::Url::parse(&database).unwrap();
    database_url.set_username(&username).unwrap();
    database_url.set_password(None).unwrap();
    let mut config: StoreConfig = serde_json::from_value(json!({
        "database": {"url": database_url.as_str(), "password": password},
        "redis": config_connection(url::Url::parse(&redis_url).unwrap()),
    }))
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let result = async {
        config.resolve_and_validate(directory.path())?;
        let bundle = gateway_store::initialize_read_only(config).await?;
        let mut states = Vec::new();
        for probe in bundle.health_probes() {
            states.push(probe.check().await);
        }
        Ok::<_, gateway_store::StoreError>(states)
    }
    .await;
    // 连接失败时也先清理临时角色，避免回归断言留下测试账号
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP ROLE {username}")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    let states = result.expect("PostgreSQL password percent sequences remain literal");
    assert!(!states.is_empty());
    assert!(
        states
            .iter()
            .all(|state| *state == gateway_core::health::HealthState::Healthy)
    );
}
