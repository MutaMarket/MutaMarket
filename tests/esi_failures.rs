//! Behavior tests for the captured detail of failed ESI requests: what
//! is recorded, what is deliberately never recorded (tokens, unlisted
//! request bodies), and what bounds the table.
//!
//! ESI is replaced by a local mock server on an ephemeral port; our own
//! code is never stubbed.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::StatusCode;
use axum::http::{Method, Request, header};
use axum::routing::{get, post};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::esi::failures::{
    BODY_CAPTURE_BYTES, CAPTURES_PER_MINUTE_PER_KIND, ESI_CALLER, EsiCaller, FAILURE_HISTORY_KEEP,
    FAILURE_RETENTION_DAYS,
};
use sqlx::{PgPool, Row};

/// The shape ESI answers a failure with.
const ESI_ERROR_BODY: &str = r#"{"error": "Internal error", "timeout": 5}"#;

/// A token that appears nowhere else, so a single query can prove it was
/// never persisted.
const CANARY_TOKEN: &str = "canary-access-token-3f9a1c";

/// The Forge, for the market-history endpoint the mock stands in for.
const FORGE_REGION_ID: i64 = 10_000_002;
const PLEX_TYPE_ID: i64 = 44_992;

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

/// A pool with the failure table emptied, so each test owns its rows.
async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    sqlx::query("delete from esi_failures")
        .execute(&pool)
        .await
        .expect("clean failures");
    pool
}

async fn failures(pool: &PgPool) -> Vec<sqlx::postgres::PgRow> {
    sqlx::query("select * from esi_failures order by id")
        .fetch_all(pool)
        .await
        .expect("read failures")
}

async fn a_failed_response_is_captured_with_what_explains_it() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [
                    ("content-type", "application/json"),
                    ("x-esi-error-limit-remain", "42"),
                    ("x-esi-error-limit-reset", "17"),
                    // Never in the allowlist, so it must not be stored.
                    ("set-cookie", "session=nope"),
                ],
                ESI_ERROR_BODY,
            )
        }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    assert!(
        esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID)
            .await
            .is_err()
    );

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.get::<String, _>("endpoint"), "markets/history");
    assert_eq!(row.get::<String, _>("method"), "GET");
    assert_eq!(row.get::<Option<i32>, _>("status"), Some(500));
    assert_eq!(row.get::<Option<String>, _>("error_kind"), None);
    assert_eq!(
        row.get::<Option<String>, _>("error_message").as_deref(),
        Some("Internal error"),
        "ESI's own message is lifted out of the body",
    );
    assert!(!row.get::<bool, _>("authenticated"));
    assert!(
        row.get::<Option<String>, _>("url")
            .unwrap()
            .contains("history")
    );
    assert!(row.get::<i64, _>("duration_ms") >= 0);
    assert_eq!(
        row.get::<Option<i64>, _>("response_bytes"),
        Some(ESI_ERROR_BODY.len() as i64),
        "the length before truncation",
    );

    // Only the allowlisted headers, so nothing incidental rides along.
    let headers = row
        .get::<Option<serde_json::Value>, _>("response_headers")
        .expect("headers");
    let mut names: Vec<&str> = headers
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "content-type",
            "x-esi-error-limit-remain",
            "x-esi-error-limit-reset"
        ],
    );
}

async fn a_request_that_gets_no_response_is_captured_as_a_transport_failure() {
    let pool = setup().await;

    // Bind and drop, so the address is dead but routable.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("address");
    drop(listener);

    let esi = EsiClient::new(&format!("http://{address}")).with_failure_log(pool.clone());
    assert!(
        esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID)
            .await
            .is_err()
    );

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 1, "the class with no body is still inspectable");
    let row = &rows[0];
    assert_eq!(row.get::<Option<i32>, _>("status"), None);
    assert_eq!(
        row.get::<Option<String>, _>("error_kind").as_deref(),
        Some("connect"),
    );
    assert!(row.get::<Option<String>, _>("error_message").is_some());
    assert_eq!(row.get::<Option<String>, _>("response_body"), None);
}

async fn an_access_token_is_never_persisted() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/characters/{character}/assets/",
        get(|| async {
            (
                StatusCode::UNAUTHORIZED,
                // Even if ESI echoed the token back at us, it must not
                // survive into the table.
                format!(r#"{{"error": "token {CANARY_TOKEN} is invalid"}}"#),
            )
        }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    assert!(esi.character_assets(CANARY_TOKEN, 42, 1).await.is_err());

    let leaked: i64 = sqlx::query_scalar(
        "select count(*) from esi_failures
         where url like $1 or caller like $1 or coalesce(request_body, '') like $1
            or coalesce(response_headers::text, '') like $1",
    )
    .bind(format!("%{CANARY_TOKEN}%"))
    .fetch_one(&pool)
    .await
    .expect("scan for the token");
    assert_eq!(leaked, 0, "the bearer token must never reach the table");

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].get::<bool, _>("authenticated"),
        "that a token was sent is recorded; which one is not",
    );
}

async fn a_public_call_is_not_marked_authenticated() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::BAD_GATEWAY, "upstream is down") }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].get::<bool, _>("authenticated"));
    assert_eq!(
        rows[0].get::<Option<String>, _>("error_message"),
        None,
        "a body that is not ESI's error shape yields no message",
    );
}

async fn a_long_body_is_truncated_at_the_cap() {
    let pool = setup().await;
    let oversized = "x".repeat(BODY_CAPTURE_BYTES * 3);
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(move || {
            let body = oversized.clone();
            async move { (StatusCode::INTERNAL_SERVER_ERROR, body) }
        }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    let rows = failures(&pool).await;
    // octet_length returns int4.
    let stored: i32 =
        sqlx::query_scalar("select octet_length(response_body) from esi_failures where id = $1")
            .bind(rows[0].get::<i64, _>("id"))
            .fetch_one(&pool)
            .await
            .expect("body length");

    assert_eq!(stored, BODY_CAPTURE_BYTES as i32);
    assert_eq!(
        rows[0].get::<Option<i64>, _>("response_bytes"),
        Some((BODY_CAPTURE_BYTES * 3) as i64),
        "the console can say what it is not showing",
    );
}

async fn request_bodies_are_stored_only_for_the_allowlisted_endpoints() {
    let pool = setup().await;
    let base = start_mock(
        Router::new()
            .route(
                "/latest/characters/affiliation/",
                post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "nope") }),
            )
            .route(
                "/latest/characters/{character}/mail/",
                post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "nope") }),
            ),
    )
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.affiliations(&[95_465_499, 90_000_001]).await;
    let _ = esi
        .send_mail(CANARY_TOKEN, 42, 95_465_499, "Subject", "Body text")
        .await;

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 2);

    let affiliation = rows
        .iter()
        .find(|row| row.get::<String, _>("endpoint") == "characters/affiliation")
        .expect("the affiliation failure");
    assert_eq!(
        affiliation
            .get::<Option<String>, _>("request_body")
            .as_deref(),
        Some("[95465499,90000001]"),
        "an id array is what makes the failure diagnosable",
    );

    let mail = rows
        .iter()
        .find(|row| row.get::<String, _>("endpoint") == "characters/mail")
        .expect("the mail failure");
    assert_eq!(
        mail.get::<Option<String>, _>("request_body"),
        None,
        "a player's mail is never stored: request bodies are default-deny",
    );
    assert_eq!(mail.get::<Option<i64>, _>("request_bytes"), None);
}

async fn a_burst_is_sampled_while_the_telemetry_still_counts_all_of_it() {
    let pool = setup().await;
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }
        }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let attempts = CAPTURES_PER_MINUTE_PER_KIND as usize + 7;
    for _ in 0..attempts {
        let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;
    }

    assert_eq!(
        hits.load(Ordering::Relaxed),
        attempts,
        "every call went out"
    );
    assert_eq!(
        failures(&pool).await.len(),
        CAPTURES_PER_MINUTE_PER_KIND as usize,
        "one storm must not evict the rest of the table",
    );

    // The exact counts stay in the telemetry, which is what lets the
    // console say "N errors this minute, M captured".
    let counted: u64 = esi
        .telemetry()
        .snapshot()
        .iter()
        .flat_map(|bucket| bucket.endpoints.values())
        .map(|counts| counts.server_errors)
        .sum();
    assert_eq!(counted, attempts as u64);
}

async fn the_table_is_bounded_by_row_count() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;

    // Seeding in SQL rather than through the client is deliberate: the
    // sampler caps real captures at a handful per minute.
    sqlx::query(
        "insert into esi_failures
             (occurred_at, endpoint, method, url, status, duration_ms, authenticated)
         select now(), 'seeded', 'GET', 'https://esi/seeded', 500, 1, false
         from generate_series(1, $1)",
    )
    .bind((FAILURE_HISTORY_KEEP + 5) as i32)
    .execute(&pool)
    .await
    .expect("seed rows");

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    let total: i64 = sqlx::query_scalar("select count(*) from esi_failures")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(total, FAILURE_HISTORY_KEEP);

    // The newest capture is the one that survived, not an evicted one.
    let newest: String =
        sqlx::query_scalar("select endpoint from esi_failures order by id desc limit 1")
            .fetch_one(&pool)
            .await
            .expect("newest");
    assert_eq!(newest, "markets/history");
}

async fn the_table_is_bounded_by_age() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;

    // Well inside the row cap, so only age can remove it.
    sqlx::query(
        "insert into esi_failures
             (occurred_at, endpoint, method, url, status, duration_ms, authenticated)
         values (now() - make_interval(days => $1::int), 'stale', 'GET',
                 'https://esi/stale', 500, 1, false)",
    )
    .bind((FAILURE_RETENTION_DAYS + 1) as i32)
    .execute(&pool)
    .await
    .expect("seed a stale row");

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    let endpoints: Vec<String> =
        sqlx::query_scalar("select endpoint from esi_failures order by id")
            .fetch_all(&pool)
            .await
            .expect("endpoints");
    assert_eq!(endpoints, ["markets/history"], "age prunes on its own");
}

async fn the_calling_job_is_recorded_with_its_run() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    ESI_CALLER
        .scope(EsiCaller::job("market-histories", Some(4242)), async {
            let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;
        })
        .await;

    let rows = failures(&pool).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get::<Option<String>, _>("caller").as_deref(),
        Some("job:market-histories"),
    );
    assert_eq!(
        rows[0].get::<Option<i64>, _>("scheduler_run_id"),
        Some(4242)
    );
}

async fn a_failure_raised_while_handling_a_request_names_the_route() {
    let pool = setup().await;
    // The appraise endpoint resolves a dynamic item through ESI, so a
    // failing mock reaches the client from inside a handler.
    let base = start_mock(Router::new().route(
        "/latest/dogma/dynamic/items/{type}/{item}/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;
    unsafe { std::env::set_var("ESI_BASE_URL", &base) };

    let app = mutamarket::server::test_router().await;
    let request = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/modules")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({ "type_id": 47800, "item_id": 1 }).to_string(),
        ))
        .expect("valid request");
    let _ = tower::ServiceExt::oneshot(app, request).await;

    let callers: Vec<Option<String>> =
        sqlx::query_scalar("select caller from esi_failures order by id")
            .fetch_all(&pool)
            .await
            .expect("callers");
    assert!(
        callers
            .iter()
            .any(|caller| caller.as_deref() == Some("http:POST /modules")),
        "a handler-driven failure names the route that raised it: {callers:?}",
    );
}

async fn a_call_outside_any_job_or_handler_records_no_caller() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;

    let esi = EsiClient::new(&base).with_failure_log(pool.clone());
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    let rows = failures(&pool).await;
    assert_eq!(rows[0].get::<Option<String>, _>("caller"), None);
    assert_eq!(rows[0].get::<Option<i64>, _>("scheduler_run_id"), None);
}

async fn a_client_without_a_failure_log_captures_nothing() {
    let pool = setup().await;
    let base = start_mock(Router::new().route(
        "/latest/markets/{region}/history/",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "{}") }),
    ))
    .await;

    // The many test constructions of EsiClient::new must stay free of
    // database writes.
    let esi = EsiClient::new(&base);
    let _ = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await;

    assert!(failures(&pool).await.is_empty());
}

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

async fn send(app: &Router, path: &str, session: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().method(Method::GET).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let response = tower::ServiceExt::oneshot(app.clone(), builder.body(Body::empty()).unwrap())
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
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
    mutamarket::auth::session::create_session(pool, user_id, None)
        .await
        .expect("create session")
}

/// The admin surface over the captured failures: the gate, the exact key
/// sets, the filters and their refusals.
async fn the_admin_endpoints_serve_and_gate_the_captured_failures() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;
    let admin = seed_user(&pool, "Failure Admin", true).await;
    let pleb = seed_user(&pool, "Failure Pleb", false).await;

    // One captured failure to read back, seeded directly so the shape is
    // exactly known.
    let id: i64 = sqlx::query_scalar(
        "insert into esi_failures
             (occurred_at, endpoint, method, url, status, error_message, duration_ms,
              authenticated, caller, scheduler_run_id, response_headers, response_body,
              response_bytes, request_body, request_bytes)
         values (now(), 'contracts/public', 'GET', 'https://esi/contracts', 500,
                 'Internal error', 12, false, 'job:region-contracts', 7,
                 '{\"content-type\": \"application/json\"}'::jsonb,
                 '{\"error\": \"Internal error\"}', 29, null, null)
         returning id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed a failure");

    for path in [
        "/api/admin/esi-failures",
        &format!("/api/admin/esi-failures/{id}"),
    ] {
        let (status, error) = send(&app, path, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(error["message"], serde_json::json!("Unauthenticated."));

        let (status, error) = send(&app, path, Some(&pleb)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(error["message"], serde_json::json!("Forbidden."));
    }

    let (status, body) = send(&app, "/api/admin/esi-failures", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["failures"]);
    assert_eq!(
        sorted_keys(&body["failures"][0]),
        [
            "authenticated",
            "caller",
            "duration_ms",
            "endpoint",
            "error_kind",
            "error_message",
            "id",
            "method",
            "occurred_at",
            "status",
            "url",
        ],
    );

    let (status, detail) = send(&app, &format!("/api/admin/esi-failures/{id}"), Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&detail),
        [
            "authenticated",
            "caller",
            "duration_ms",
            "endpoint",
            "error_kind",
            "error_message",
            "id",
            "method",
            "occurred_at",
            "request_body",
            "request_bytes",
            "response_body",
            "response_bytes",
            "response_headers",
            "scheduler_run_id",
            "status",
            "url",
        ],
        "the detail is the summary plus what does not fit on a poll",
    );
    assert_eq!(detail["scheduler_run_id"], serde_json::json!(7));
    assert_eq!(
        detail["response_headers"]["content-type"],
        serde_json::json!("application/json"),
    );

    let (status, error) = send(&app, "/api/admin/esi-failures/999999999", Some(&admin)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["message"], serde_json::json!("Unknown failure."));

    // Filters narrow rather than silently ignoring what they cannot do.
    let (_, body) = send(&app, "/api/admin/esi-failures?class=server", Some(&admin)).await;
    assert_eq!(body["failures"].as_array().expect("array").len(), 1);
    let (_, body) = send(
        &app,
        "/api/admin/esi-failures?class=transport",
        Some(&admin),
    )
    .await;
    assert!(body["failures"].as_array().expect("array").is_empty());
    let (_, body) = send(
        &app,
        "/api/admin/esi-failures?endpoint=characters/assets",
        Some(&admin),
    )
    .await;
    assert!(body["failures"].as_array().expect("array").is_empty());

    let (status, error) = send(&app, "/api/admin/esi-failures?class=bogus", Some(&admin)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error["message"],
        serde_json::json!("Unknown failure class: bogus."),
    );

    // The live section carries the summaries and the bounds it is under.
    let (status, body) = send(&app, "/api/admin/live?sections=failures", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["failures"]);
    assert_eq!(
        sorted_keys(&body["failures"]),
        ["captured", "keep", "retention_days"],
    );
    assert_eq!(
        body["failures"]["keep"],
        serde_json::json!(FAILURE_HISTORY_KEEP),
    );
    assert_eq!(
        body["failures"]["retention_days"],
        serde_json::json!(FAILURE_RETENTION_DAYS),
    );
}

/// One test, run in sequence: every case here asserts over the whole
/// `esi_failures` table, and the suite shares one database, so parallel
/// test runtimes would delete each other's rows mid-assertion.
#[tokio::test]
async fn esi_failures_are_captured_bounded_and_free_of_secrets() {
    a_failed_response_is_captured_with_what_explains_it().await;
    a_request_that_gets_no_response_is_captured_as_a_transport_failure().await;
    an_access_token_is_never_persisted().await;
    a_public_call_is_not_marked_authenticated().await;
    a_long_body_is_truncated_at_the_cap().await;
    request_bodies_are_stored_only_for_the_allowlisted_endpoints().await;
    a_burst_is_sampled_while_the_telemetry_still_counts_all_of_it().await;
    the_table_is_bounded_by_row_count().await;
    the_table_is_bounded_by_age().await;
    the_calling_job_is_recorded_with_its_run().await;
    a_failure_raised_while_handling_a_request_names_the_route().await;
    a_call_outside_any_job_or_handler_records_no_caller().await;
    the_admin_endpoints_serve_and_gate_the_captured_failures().await;
    a_client_without_a_failure_log_captures_nothing().await;
}
