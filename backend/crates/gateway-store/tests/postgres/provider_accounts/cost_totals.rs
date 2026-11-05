//! 验证账号累计消耗账本跨越请求明细清理持续累计，并与源表真值一致

use super::super::drain_analytics;
use super::*;

#[tokio::test]
async fn account_cost_total_should_survive_detail_purge_and_follow_amount_changes() {
    let Some(database) = TestDatabase::create("account_cost_total_ledger").await else {
        return;
    };
    let repository = PgProviderAccountRepository::new(database.pool.clone());
    for id in ["acct_cost_a", "acct_cost_b"] {
        repository
            .insert_provider_account(account(id, id))
            .await
            .unwrap();
    }
    let start = Utc::now() - TimeDelta::minutes(30);
    for (id, account_id, tokens, cost) in [
        ("cost_a_keep", "acct_cost_a", 11, "1.5"),
        ("cost_a_purge", "acct_cost_a", 23, "2.5"),
        ("cost_b_keep", "acct_cost_b", 7, "4.25"),
    ] {
        seed_model_request(
            &database.pool,
            ModelRequestSeed {
                request_id: id,
                account_id,
                provider_kind: "openai",
                model: "gpt-5.6-sol",
                total_tokens: tokens,
                cost_amount: cost,
                started_at: start,
            },
        )
        .await
        .unwrap();
    }
    // 未完成与未计价的请求不进入账本
    sqlx::query(
        "insert into model_requests (id, client_api_key_ref, operation, client_transport, provider_kind, provider_account_ref, outcome, started_at, recovery_lease_expires_at, request_observation_json)
         values ('cost_running','key-provider-account-test','responses','http_sse','openai','acct_cost_a','running',$1,$1 + interval '5 minutes', jsonb_build_object('request',jsonb_build_object('configRevision',1,'protocol','openai','endpoint','/v1/responses','compact',false),'routing',jsonb_build_object('scope','all','groupRefs','{}'::text[],'groupNamesSnapshot','[]'::jsonb)))",
    )
    .bind(start)
    .execute(&database.pool)
    .await
    .unwrap();
    drain_analytics(&database.pool).await;

    let store = admin_account_store(&database.pool);
    let costs = store.load_account_cost_total("acct_cost_a").await.unwrap();
    assert_eq!(costs.len(), 1);
    assert_eq!(costs[0].currency, "USD");
    assert_eq!(costs[0].amount.as_str(), "4");

    // 独立源表真值不经过账本触发器，防止仅比较两个相同错误的读取分支
    let truth: String = sqlx::query_scalar(
        "select coalesce(sum(cost_amount),0)::text from model_requests where provider_account_ref='acct_cost_a' and outcome='succeeded' and downstream_committed_at is not null",
    )
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(
        truth.parse::<gateway_store::DecimalAmount>().unwrap(),
        costs[0].amount
    );

    // 改写金额只调整账本差额
    sqlx::query("update model_requests set cost_amount=3.75 where id='cost_a_purge'")
        .execute(&database.pool)
        .await
        .unwrap();
    let repriced = store.load_account_cost_total("acct_cost_a").await.unwrap();
    assert_eq!(repriced[0].amount.as_str(), "5.25");

    // 换绑账号把已记金额转到新归属
    sqlx::query(
        "update model_requests set provider_account_ref='acct_cost_b' where id='cost_a_purge'",
    )
    .execute(&database.pool)
    .await
    .unwrap();
    assert_eq!(
        store.load_account_cost_total("acct_cost_a").await.unwrap()[0]
            .amount
            .as_str(),
        "1.5"
    );
    assert_eq!(
        store.load_account_cost_total("acct_cost_b").await.unwrap()[0]
            .amount
            .as_str(),
        "8"
    );

    // 明细被清理后累计不回退，这是账本相对用量聚合的唯一区别
    sqlx::query("delete from model_requests where id in ('cost_a_keep','cost_a_purge')")
        .execute(&database.pool)
        .await
        .unwrap();
    let retained = store.load_account_cost_total("acct_cost_a").await.unwrap();
    assert_eq!(
        retained[0].amount.as_str(),
        "1.5",
        "purged detail must not reduce the accumulated total"
    );
    assert_eq!(
        store.load_account_cost_total("acct_cost_b").await.unwrap()[0]
            .amount
            .as_str(),
        "8"
    );

    // 没有账本记录的账号返回空列表，不视为错误
    assert!(
        store
            .load_account_cost_total("acct_cost_missing")
            .await
            .unwrap()
            .is_empty()
    );

    database.close().await;
}
