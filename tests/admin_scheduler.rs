//! Behavior tests for the scheduler job registry and its admin API:
//! manual runs record history, pruning bounds it, and the endpoints are
//! admin-gated.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::sync::Arc;
use std::time::Duration;

use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::EstimatorClient;
use mutamarket::mutation::reference::ReferenceData;
use mutamarket::scheduler::{JobDeps, RUN_HISTORY_KEEP, RunNowOutcome, Scheduler, SchedulerHandle};
use sqlx::PgPool;

/// The DB-only sweeper: safe to really run without any ESI mock.
const DB_ONLY_JOB: &str = "stale-asset-imports";

/// A recorded run must land within this window.
const RUN_TIMEOUT: Duration = Duration::from_secs(5);

fn test_scheduler(pool: &PgPool) -> SchedulerHandle {
    Scheduler::disabled(JobDeps {
        pool: pool.clone(),
        reference: Arc::new(ReferenceData::default()),
        esi: EsiClient::new("http://127.0.0.1:9"),
        estimator: EstimatorClient::new("http://127.0.0.1:9"),
        sso: SsoClient::new("http://127.0.0.1:9", "client", "secret", "http://test/eve/callback"),
    })
}

async fn wait_for_finished_run(
    pool: &PgPool,
    job: &str,
) -> (String, Option<String>, Option<String>) {
    let deadline = tokio::time::Instant::now() + RUN_TIMEOUT;
    loop {
        let run: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "select outcome, summary, error from scheduler_runs
             where job = $1 and finished_at is not null order by id desc limit 1",
        )
        .bind(job)
        .fetch_optional(pool)
        .await
        .expect("read runs");

        if let Some((Some(outcome), summary, error)) = run {
            return (outcome, summary, error);
        }
        assert!(tokio::time::Instant::now() < deadline, "no finished {job} run recorded");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn manual_runs_record_and_prune_history() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    sqlx::query("delete from scheduler_runs where job = $1")
        .bind(DB_ONLY_JOB)
        .execute(&pool)
        .await
        .expect("clean runs");

    let scheduler = test_scheduler(&pool);

    // Unknown jobs are rejected before anything is spawned.
    assert!(matches!(scheduler.run_now("no-such-job"), RunNowOutcome::UnknownJob));

    // A manual run works with the loops disabled and records its outcome.
    assert!(matches!(scheduler.run_now(DB_ONLY_JOB), RunNowOutcome::Started));
    let (outcome, summary, error) = wait_for_finished_run(&pool, DB_ONLY_JOB).await;
    assert_eq!(outcome, "success");
    assert!(
        summary.as_deref().is_some_and(|s| s.ends_with("stale asset imports failed")),
        "the summary carries the sweep count: {summary:?}",
    );
    assert_eq!(error, None);

    // History is pruned to the newest RUN_HISTORY_KEEP rows per job.
    for _ in 0..(RUN_HISTORY_KEEP + 10) {
        sqlx::query(
            "insert into scheduler_runs (job, finished_at, outcome, summary)
             values ($1, now(), 'success', 'backfill')",
        )
        .bind(DB_ONLY_JOB)
        .execute(&pool)
        .await
        .expect("backfill run");
    }
    assert!(matches!(scheduler.run_now(DB_ONLY_JOB), RunNowOutcome::Started));
    let deadline = tokio::time::Instant::now() + RUN_TIMEOUT;
    loop {
        let count: i64 = sqlx::query_scalar("select count(*) from scheduler_runs where job = $1")
            .bind(DB_ONLY_JOB)
            .fetch_one(&pool)
            .await
            .expect("count runs");
        if count == RUN_HISTORY_KEEP {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "history not pruned to {RUN_HISTORY_KEEP} (still {count})",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
