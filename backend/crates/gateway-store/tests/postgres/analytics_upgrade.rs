//! 验证统计升级保留历史事实与回填进度，并重建已有时间桶

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{TEST_MIGRATOR, TestDatabase};

#[tokio::test]
async fn analytics_upgrade_preserves_facts_and_backfill_progress_and_requeues_buckets() {
    let Some(database) = TestDatabase::create_through("analytics_upgrade", 29).await else {
        return;
    };
    sqlx::raw_sql(
        "insert into request_analytics_facts(id,started_at,source_version) values
            ('first','2026-10-01 10:01:00+00',1),
            ('same_bucket','2026-10-01 10:02:00+00',2),
            ('next_bucket','2026-10-01 10:16:00+00',3);
         insert into request_analytics_pending_buckets(bucket_start,oldest_pending_at)
            values ('2026-10-01 10:00:00+00','2026-10-01 09:00:00+00');
         update request_analytics_build_state set
            backfill_complete=true, backfill_cursor_id='next_bucket',
            backfill_cursor_started_at='2026-10-01 10:16:00+00',
            coverage_start='2026-10-01 10:00:00+00',
            coverage_end='2026-10-01 10:30:00+00', generation=7;",
    )
    .execute(&database.pool)
    .await
    .unwrap();
    let facts_before: Vec<Value> =
        sqlx::query_scalar("select to_jsonb(f) from request_analytics_facts f order by id")
            .fetch_all(&database.pool)
            .await
            .unwrap();
    let progress_before: Value =
        sqlx::query_scalar("select to_jsonb(s) from request_analytics_build_state s")
            .fetch_one(&database.pool)
            .await
            .unwrap();
    let pending_before: (i64, DateTime<Utc>) = sqlx::query_as(
        "select change_version,oldest_pending_at from request_analytics_pending_buckets",
    )
    .fetch_one(&database.pool)
    .await
    .unwrap();

    TEST_MIGRATOR.run(&database.pool).await.unwrap();

    let facts_after: Vec<Value> =
        sqlx::query_scalar("select to_jsonb(f) from request_analytics_facts f order by id")
            .fetch_all(&database.pool)
            .await
            .unwrap();
    let progress_after: Value =
        sqlx::query_scalar("select to_jsonb(s) from request_analytics_build_state s")
            .fetch_one(&database.pool)
            .await
            .unwrap();
    assert_eq!(facts_after, facts_before);
    assert_eq!(progress_after, progress_before);
    let pending: Vec<(DateTime<Utc>, i64, DateTime<Utc>)> = sqlx::query_as(
        "select bucket_start,change_version,oldest_pending_at
         from request_analytics_pending_buckets order by bucket_start",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    let expected_buckets = ["2026-10-01T10:00:00Z", "2026-10-01T10:15:00Z"]
        .map(|value| value.parse::<DateTime<Utc>>().unwrap());
    assert_eq!(
        pending.iter().map(|row| row.0).collect::<Vec<_>>(),
        expected_buckets
    );
    assert!(pending[0].1 > pending_before.0);
    assert_eq!(pending[0].2, pending_before.1);

    // 重启不会再次入队，也不会覆盖最早待处理时间
    TEST_MIGRATOR.run(&database.pool).await.unwrap();
    let pending_after_restart: Vec<(DateTime<Utc>, i64, DateTime<Utc>)> = sqlx::query_as(
        "select bucket_start,change_version,oldest_pending_at
         from request_analytics_pending_buckets order by bucket_start",
    )
    .fetch_all(&database.pool)
    .await
    .unwrap();
    assert_eq!(pending_after_restart, pending);
    database.close().await;
}
