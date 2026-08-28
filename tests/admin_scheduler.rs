//! Behavior tests for the scheduler job registry and its admin API:
//! manual runs record history, pruning bounds it, and the endpoints are
//! admin-gated.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::mutation::reference::ReferenceData;
use mutamarket::scheduler::{JobDeps, RUN_HISTORY_KEEP, RunNowOutcome, Scheduler, SchedulerHandle};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

/// The DB-only sweeper: safe to really run without any ESI mock.
const DB_ONLY_JOB: &str = "stale-asset-imports";

/// A recorded run must land within this window.
const RUN_TIMEOUT: Duration = Duration::from_secs(5);

fn test_scheduler(pool: &PgPool) -> SchedulerHandle {
    Scheduler::disabled(JobDeps {
        pool: pool.clone(),
        reference: Arc::new(ReferenceData::default()),
        esi: EsiClient::new("http://127.0.0.1:9"),
        estimator: Estimator::new(),
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

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> =
        value.as_object().expect("a JSON object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("valid request");

    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

/// A fresh user with a session; admin at will. Idempotent per name.
async fn seed_user(pool: &PgPool, name: &str, is_admin: bool) -> String {
    sqlx::query("delete from users where name = $1")
        .bind(name)
        .execute(pool)
        .await
        .expect("clean user");
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name, is_admin) values ($1, $2) returning id")
            .bind(name)
            .bind(is_admin)
            .fetch_one(pool)
            .await
            .expect("create user");

    create_session(pool, user_id, None).await.expect("create session")
}

#[tokio::test]
async fn admin_api_gates_and_serves_the_scheduler() {
    let app = mutamarket::server::test_router().await;
    let pool = db::test_pool().await.expect("Postgres reachable");

    let admin = seed_user(&pool, "Scheduler Admin", true).await;
    let pleb = seed_user(&pool, "Scheduler Pleb", false).await;

    // Non-admin users are turned away everywhere.
    for (method, path, body) in [
        (Method::GET, "/api/admin/scheduler", None),
        (Method::GET, "/api/admin/service-character", None),
        (Method::POST, "/api/admin/scheduler/stale-asset-imports/run", None),
        (
            Method::PUT,
            "/api/admin/scheduler/stale-asset-imports",
            Some(json!({"paused": true})),
        ),
    ] {
        let (status, error) = send(&app, method, path, Some(&pleb), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(error["message"], json!("Forbidden."));
    }

    // The status payload carries every job with the exact key sets.
    let (status, body) = send(&app, Method::GET, "/api/admin/scheduler", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["database", "enabled", "in_downtime", "jobs"]);
    assert_eq!(body["enabled"], json!(false), "test routers never start the loops");
    assert_eq!(
        sorted_keys(&body["database"]),
        [
            "assets",
            "characters",
            "contract_items",
            "contracts",
            "market_history_days",
            "modules",
            "modules_without_estimate",
            "public_ownerships",
            "users",
        ],
    );
    let jobs = body["jobs"].as_array().expect("jobs array");
    let job_names: Vec<&str> =
        jobs.iter().map(|job| job["name"].as_str().expect("name")).collect();
    assert_eq!(
        job_names,
        [
            "character-contracts",
            "character-assets",
            "stale-asset-imports",
        "statistics-views",
            "structures",
            "alliances",
            "market-histories",
            "region-contracts",
            "character-names",
            "auction-bids",
            "estimates",
            "training-modules",
            "metric-samples",
            "offer-notifications",
            "notification-delivery",
            "eve-mails",
            "launcher-ads",
            "estimator-training",
        ],
    );
    for job in jobs {
        assert_eq!(
            sorted_keys(job),
            [
                "downtime_guarded",
                "interval_seconds",
                "last_runs",
                "name",
                "next_run_at",
                "paused",
                "progress",
                "running",
            ],
        );
        for run in job["last_runs"].as_array().expect("runs array") {
            assert_eq!(
                sorted_keys(run),
                [
                    "duration_seconds",
                    "error",
                    "finished_at",
                    "items",
                    "metrics",
                    "outcome",
                    "started_at",
                    "summary",
                ],
            );
        }
    }

    // The telemetry payload carries the bucket window; the test router's
    // client has made no ESI requests, so the window is empty here.
    let (status, body) = send(&app, Method::GET, "/api/admin/telemetry", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["buckets", "window_minutes"]);
    assert_eq!(body["window_minutes"], json!(60));
    assert!(body["buckets"].as_array().expect("buckets array").is_empty());

    // Pausing persists to scheduler_jobs and reflects in the payload.
    let (status, _) = send(
        &app,
        Method::PUT,
        "/api/admin/scheduler/character-names",
        Some(&admin),
        Some(json!({"paused": true})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let persisted: bool =
        sqlx::query_scalar("select paused from scheduler_jobs where job = 'character-names'")
            .fetch_one(&pool)
            .await
            .expect("paused row");
    assert!(persisted);
    let (_, body) = send(&app, Method::GET, "/api/admin/scheduler", Some(&admin), None).await;
    let job = body["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .find(|job| job["name"] == json!("character-names"))
        .expect("job listed");
    assert_eq!(job["paused"], json!(true));
    let (status, _) = send(
        &app,
        Method::PUT,
        "/api/admin/scheduler/character-names",
        Some(&admin),
        Some(json!({"paused": false})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Invalid payloads and unknown jobs carry their statuses.
    let (status, error) = send(
        &app,
        Method::PUT,
        "/api/admin/scheduler/character-names",
        Some(&admin),
        Some(json!({"paused": "sideways"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["message"], json!("The given data was invalid."));
    let (status, error) =
        send(&app, Method::POST, "/api/admin/scheduler/no-such-job/run", Some(&admin), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["message"], json!("Unknown job."));

    // A triggered run answers 202 and lands in the recorded history.
    sqlx::query("delete from scheduler_runs where job = $1")
        .bind(DB_ONLY_JOB)
        .execute(&pool)
        .await
        .expect("clean runs");
    let (status, message) = send(
        &app,
        Method::POST,
        "/api/admin/scheduler/stale-asset-imports/run",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(message["message"], json!("Run started."));
    let (outcome, _, _) = wait_for_finished_run(&pool, DB_ONLY_JOB).await;
    assert_eq!(outcome, "success");
}

#[tokio::test]
async fn historic_contract_update_gates_and_edits() {
    let app = mutamarket::server::test_router().await;
    let pool = db::test_pool().await.expect("Postgres reachable");

    let admin = seed_user(&pool, "History Admin", true).await;
    let pleb = seed_user(&pool, "History Pleb", false).await;

    const CONTRACT: i64 = 800_401;
    sqlx::query("insert into characters (id, name) values (90999997, 'Edit Issuer') on conflict (id) do nothing")
        .execute(&pool)
        .await
        .expect("seed issuer");
    sqlx::query("delete from historic_contracts where id = $1")
        .bind(CONTRACT)
        .execute(&pool)
        .await
        .expect("clean contract");
    sqlx::query(
        "insert into historic_contracts
             (id, status, region_id, issuer_id, type, unified_price, abyssal_modules_count)
         values ($1, 'completed', 10000002, 90999997, 'item_exchange', 100000000, 1)",
    )
    .bind(CONTRACT)
    .execute(&pool)
    .await
    .expect("seed contract");

    let path = format!("/api/historic-contracts/{CONTRACT}");
    let (status, error) = send(&app, Method::PUT, &path, None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["message"], json!("Unauthenticated."));
    let (status, error) = send(&app, Method::PUT, &path, Some(&pleb), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["message"], json!("Forbidden."));

    let (status, error) = send(
        &app,
        Method::PUT,
        "/api/historic-contracts/999999901",
        Some(&admin),
        Some(json!({"status": "failed"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["message"], json!("Not found."));

    let (status, error) = send(
        &app,
        Method::PUT,
        &path,
        Some(&admin),
        Some(json!({"status": "sideways"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["message"], json!("The given data was invalid."));

    // A partial update touches exactly the sent fields.
    let (status, _) = send(
        &app,
        Method::PUT,
        &path,
        Some(&admin),
        Some(json!({"ignore_for_training": true, "non_abyssal_modules_count": 600})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (ignored, non_abyssal, contract_status): (bool, i32, String) = sqlx::query_as(
        "select ignore_for_training, non_abyssal_modules_count, status
         from historic_contracts where id = $1",
    )
    .bind(CONTRACT)
    .fetch_one(&pool)
    .await
    .expect("updated row");
    assert!(ignored);
    assert_eq!(non_abyssal, 600);
    assert_eq!(contract_status, "completed", "untouched fields keep their value");

    sqlx::query("delete from historic_contracts where id = $1")
        .bind(CONTRACT)
        .execute(&pool)
        .await
        .expect("clean contract");
}

#[tokio::test]
async fn metric_samples_record_and_the_system_endpoint_answers() {
    let app = mutamarket::server::test_router().await;
    let pool = db::test_pool().await.expect("Postgres reachable");
    let admin = seed_user(&pool, "System Admin", true).await;

    // Every readable Recordable lands one sample (the system readings
    // are unavailable outside Linux); the status payload then serves
    // the series keyed by metric name.
    let esi = EsiClient::new("http://127.0.0.1:9");
    let context = mutamarket::metrics::SampleContext { pool: &pool, esi: &esi };
    let (written, skipped) = mutamarket::metrics::record_all(&context).await.expect("metrics record");
    assert_eq!(written + skipped, mutamarket::metrics::REGISTRY.len());
    assert!(written >= 1, "the database size always records: {written}");

    // The windowed history endpoint: admin-gated, validated window,
    // series keyed by metric.
    let (status, error) =
        send(&app, Method::GET, "/api/admin/metrics?window=1y", Some(&admin), None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["message"], json!("The selected window is invalid."));
    for window in ["24h", "3d", "7d"] {
        let (status, body) = send(
            &app,
            Method::GET,
            &format!("/api/admin/metrics?window={window}"),
            Some(&admin),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(sorted_keys(&body), ["series", "step_seconds", "window"]);
        assert_eq!(body["window"], json!(window));
        let series = body["series"]["database_size_bytes"].as_array().expect("series");
        assert!(!series.is_empty(), "database size has samples in {window}");
        assert_eq!(sorted_keys(series.last().expect("sample")), ["taken_at", "value"]);
    }

    // The system endpoint: admin-gated, exact key set; the Linux-only
    // readings may be null on the dev host.
    let (status, error) = send(&app, Method::GET, "/api/admin/system", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["message"], json!("Unauthenticated."));
    let (status, body) = send(&app, Method::GET, "/api/admin/system", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        [
            "cpu_cores",
            "cpu_seconds",
            "database_size_bytes",
            "disk_total_bytes",
            "disk_used_bytes",
            "memory_current_bytes",
            "memory_limit_bytes",
            "memory_rss_bytes",
            "memory_total_bytes",
            "network_rx_bytes",
            "network_tx_bytes",
            "uptime_seconds",
        ],
    );
    assert!(body["database_size_bytes"].as_i64().expect("db size") > 0);
    assert!(body["cpu_cores"].as_i64().expect("cores") > 0);

    // The service-character card payload (value depends on whether an
    // authorize flow or env fallback configured one).
    let (status, body) =
        send(&app, Method::GET, "/api/admin/service-character", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["character", "source"]);
    if let Some(character) = body["character"].as_object() {
        let mut keys: Vec<&str> = character.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["id", "name", "scopes"]);
    }
}
