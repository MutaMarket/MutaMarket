//! Behavior tests for request-activity tracking: what the middleware
//! counts, what it must never count, and the aggregation the console
//! reads back.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::activity::{ActivityRecorder, flush};
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::mutation::reference::ReferenceData;
use mutamarket::scheduler::{JobDeps, Scheduler};
use serde_json::json;
use sqlx::{PgPool, Row};
use tower::ServiceExt;

/// Users the suite owns, so it can clean exactly its own rows.
const BUSY_USER: &str = "ZZ Activity Busy";
const QUIET_USER: &str = "ZZ Activity Quiet";

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    sqlx::query("delete from activity_hours")
        .execute(&pool)
        .await
        .expect("clean hours");
    sqlx::query(
        "delete from user_activity_days where user_id in
             (select id from users where name in ($1, $2))",
    )
    .bind(BUSY_USER)
    .bind(QUIET_USER)
    .execute(&pool)
    .await
    .expect("clean days");
    pool
}

async fn seed_user(
    pool: &PgPool,
    name: &str,
    is_admin: bool,
    created_months_ago: i32,
) -> (i64, String) {
    sqlx::query("delete from users where name = $1")
        .bind(name)
        .execute(pool)
        .await
        .expect("clean user");
    let user_id: i64 = sqlx::query_scalar(
        "insert into users (name, is_admin, created_at)
         values ($1, $2, now() - make_interval(months => $3::int)) returning id",
    )
    .bind(name)
    .bind(is_admin)
    .bind(created_months_ago)
    .fetch_one(pool)
    .await
    .expect("create user");

    let token = create_session(pool, user_id, None)
        .await
        .expect("create session");
    (user_id, token)
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

async fn get(app: &Router, path: &str, session: Option<&str>) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().method(Method::GET).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
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

/// A router sharing one recorder with a loop-less scheduler, so the test
/// can drive requests through the middleware and then flush them.
async fn router_with(pool: &PgPool, activity: Arc<ActivityRecorder>) -> Router {
    let reference = Arc::new(ReferenceData::from_tables(
        db::reference::load_reference(pool)
            .await
            .expect("reference tables load"),
    ));
    let scheduler = Scheduler::disabled(JobDeps {
        pool: pool.clone(),
        activity,
        reference: reference.clone(),
        esi: EsiClient::new("http://127.0.0.1:9"),
        estimator: Estimator::new(),
        sso: SsoClient::new(
            "http://127.0.0.1:9",
            "id",
            "secret",
            "http://test/eve/callback",
        ),
    });

    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::from_env(),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        reference,
        Some(scheduler),
    )
}

/// The middleware counts real traffic, splits it by session, and records
/// nothing at all for the console's own polls or the static paths.
async fn the_middleware_records_traffic_and_ignores_the_console() {
    let pool = setup().await;
    let activity = Arc::new(ActivityRecorder::default());
    let app = router_with(&pool, activity.clone()).await;
    let (user_id, token) = seed_user(&pool, BUSY_USER, false, 0).await;

    get(&app, "/api/nav-state", None).await;
    get(&app, "/api/nav-state", Some(&token)).await;
    get(&app, "/api/nav-state", Some(&token)).await;
    // Must leave no trace: the console polls these every five seconds.
    get(&app, "/api/admin/live", Some(&token)).await;
    get(&app, "/api/admin/esi-failures", Some(&token)).await;
    get(&app, "/img/icons/633.png", None).await;
    // An unrouted path folds into one label rather than its own route.
    get(&app, "/api/no-such-thing", None).await;

    let snapshot = activity.snapshot();
    assert_eq!(snapshot.hour.signed_in, 2);
    assert_eq!(snapshot.hour.anonymous, 2, "nav-state and the 404");
    assert_eq!(snapshot.hour.users, 1);

    let (routes, users) = flush::flush(&pool, &activity)
        .await
        .expect("flush the buffer");
    assert_eq!(users, 1);
    assert!(routes >= 2);

    let rows = sqlx::query(
        "select route, signed_in, requests from activity_hours order by route, signed_in",
    )
    .fetch_all(&pool)
    .await
    .expect("read hours");
    let mut counted: Vec<(String, bool, i64)> = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("route"),
                row.get::<bool, _>("signed_in"),
                row.get::<i64, _>("requests"),
            )
        })
        .collect();
    // Sorted here rather than in SQL: the database's collation ignores
    // punctuation, so "(not found)" would not sort where bytes put it.
    counted.sort();

    assert_eq!(
        counted,
        [
            ("GET (not found)".to_owned(), false, 1),
            ("GET /api/nav-state".to_owned(), false, 1),
            ("GET /api/nav-state".to_owned(), true, 2),
        ],
        "the console's own polls and the static path are absent entirely",
    );

    let days: i64 = sqlx::query_scalar(
        "select requests from user_activity_days where user_id = $1 and day = (now() at time zone 'UTC')::date",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("read the user day");
    assert_eq!(days, 2, "only the signed-in requests");
}

/// A second flush adds to what the first wrote rather than replacing it.
async fn flushing_twice_accumulates() {
    let pool = setup().await;
    let activity = Arc::new(ActivityRecorder::default());

    activity.record("GET /api/nav-state", None, 200, Duration::from_millis(10));
    flush::flush(&pool, &activity).await.expect("first flush");
    activity.record("GET /api/nav-state", None, 500, Duration::from_millis(30));
    flush::flush(&pool, &activity).await.expect("second flush");

    let row = sqlx::query("select requests, errors, total_ms from activity_hours")
        .fetch_one(&pool)
        .await
        .expect("one row");
    assert_eq!(row.get::<i64, _>("requests"), 2);
    assert_eq!(row.get::<i64, _>("errors"), 1);
    assert_eq!(row.get::<i64, _>("total_ms"), 40);
}

/// Counts age out of both tables on their own schedules.
async fn the_flush_prunes_what_has_aged_out() {
    let pool = setup().await;
    let activity = Arc::new(ActivityRecorder::default());
    let (user_id, _) = seed_user(&pool, QUIET_USER, false, 0).await;

    sqlx::query(
        "insert into activity_hours (hour, route, signed_in, requests)
         values (now() - interval '14 months', 'GET /old', false, 1),
                (now() - interval '12 months', 'GET /kept', false, 1)",
    )
    .execute(&pool)
    .await
    .expect("seed hours");
    sqlx::query(
        "insert into user_activity_days (user_id, day, requests)
         values ($1, (now() - interval '26 months')::date, 1),
                ($1, (now() - interval '24 months')::date, 1)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed days");

    flush::flush(&pool, &activity).await.expect("flush");

    let routes: Vec<String> = sqlx::query_scalar("select route from activity_hours order by route")
        .fetch_all(&pool)
        .await
        .expect("routes");
    assert_eq!(routes, ["GET /kept"]);

    let days: i64 =
        sqlx::query_scalar("select count(*) from user_activity_days where user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("days");
    assert_eq!(days, 1);
}

/// The endpoint is admin-only, refuses an unknown window, and answers
/// with the documented shape at every nesting level.
async fn the_activity_endpoint_is_gated_and_shaped() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;
    let (_, admin) = seed_user(&pool, BUSY_USER, true, 0).await;
    let (_, pleb) = seed_user(&pool, QUIET_USER, false, 0).await;

    let (status, error) = get(&app, "/api/admin/activity", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["message"], json!("Unauthenticated."));

    let (status, error) = get(&app, "/api/admin/activity", Some(&pleb)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["message"], json!("Forbidden."));

    let (status, error) = get(&app, "/api/admin/activity?window=nope", Some(&admin)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["message"], json!("The selected window is invalid."));

    // Seed one of everything so no list is empty.
    let user_id: i64 = sqlx::query_scalar("select id from users where name = $1")
        .bind(BUSY_USER)
        .fetch_one(&pool)
        .await
        .expect("the admin's id");
    sqlx::query(
        "insert into activity_hours (hour, route, signed_in, requests, errors, total_ms)
         values (date_trunc('hour', now()), 'GET /api/nav-state', true, 4, 1, 800)",
    )
    .execute(&pool)
    .await
    .expect("seed hours");
    sqlx::query(
        "insert into user_activity_days (user_id, day, requests)
         values ($1, (now() at time zone 'UTC')::date, 4)",
    )
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed days");

    let (status, body) = get(&app, "/api/admin/activity?window=7d", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        [
            "daily_users",
            "months",
            "routes",
            "step_seconds",
            "top_users",
            "totals",
            "traffic",
            "window",
        ],
    );
    assert_eq!(body["window"], json!("7d"));
    assert_eq!(
        sorted_keys(&body["traffic"][0]),
        ["anonymous", "at", "signed_in"]
    );
    assert_eq!(
        sorted_keys(&body["routes"][0]),
        ["average_ms", "errors", "requests", "route", "signed_in"],
    );
    assert_eq!(
        sorted_keys(&body["top_users"][0]),
        [
            "active_days",
            "created_at",
            "last_active_day",
            "name",
            "requests",
            "user_id",
        ],
    );
    assert_eq!(
        sorted_keys(&body["daily_users"][0]),
        ["day", "requests", "users"]
    );
    assert_eq!(
        sorted_keys(&body["months"][0]),
        [
            "active_users",
            "month",
            "new_users",
            "returning_users",
            "signed_up",
        ],
    );
    assert_eq!(
        sorted_keys(&body["totals"]),
        [
            "active_users",
            "new_users",
            "page_views",
            "requests",
            "signed_in_requests",
        ],
    );

    // The route roll-up's derived numbers.
    assert_eq!(body["routes"][0]["average_ms"], json!(200.0));
    assert_eq!(body["totals"]["page_views"], json!(4), "nav-state loads");
    assert_eq!(body["totals"]["signed_in_requests"], json!(4));

    // The months series always spans its full domain, so the chart's
    // x-axis does not move as data arrives.
    assert_eq!(body["months"].as_array().expect("months").len(), 24);
}

/// A user is new in the month they registered and returning after it.
async fn the_cohorts_split_new_from_returning() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;
    let (_, admin) = seed_user(&pool, BUSY_USER, true, 0).await;
    // Registered last month, so this month they are returning.
    let (returning, _) = seed_user(&pool, QUIET_USER, false, 1).await;

    sqlx::query(
        "insert into user_activity_days (user_id, day, requests)
         values ($1, (now() - interval '1 month')::date, 5),
                ($1, (now() at time zone 'UTC')::date, 3)",
    )
    .bind(returning)
    .execute(&pool)
    .await
    .expect("seed days");

    let (_, body) = get(&app, "/api/admin/activity?window=30d", Some(&admin)).await;
    let months = body["months"].as_array().expect("months");
    let this_month = months.last().expect("the current month");
    let last_month = &months[months.len() - 2];

    assert_eq!(
        last_month["new_users"],
        json!(1),
        "active in the month they registered counts as new",
    );
    assert_eq!(last_month["returning_users"], json!(0));
    assert_eq!(
        this_month["returning_users"],
        json!(1),
        "the same user, a month later, is returning",
    );
    assert_eq!(this_month["new_users"], json!(0));
    assert!(
        last_month["signed_up"].as_i64().expect("signed_up") >= 1,
        "registrations come from users.created_at alone",
    );
}

/// The live section reads the memory buffer, with no query behind it.
async fn the_live_activity_section_is_served_from_memory() {
    let pool = setup().await;
    let activity = Arc::new(ActivityRecorder::default());
    let app = router_with(&pool, activity.clone()).await;
    let (_, admin) = seed_user(&pool, BUSY_USER, true, 0).await;

    get(&app, "/api/nav-state", None).await;

    let (status, body) = get(&app, "/api/admin/live?sections=activity", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["activity"]);
    assert_eq!(
        sorted_keys(&body["activity"]),
        ["buckets", "hour", "window_minutes"],
    );
    assert_eq!(
        sorted_keys(&body["activity"]["buckets"][0]),
        ["anonymous", "minute_start", "signed_in"],
    );
    assert_eq!(
        sorted_keys(&body["activity"]["hour"]),
        ["anonymous", "requests", "signed_in", "users"],
    );
    assert_eq!(body["activity"]["hour"]["anonymous"], json!(1));
}

/// One test, run in sequence: the phases assert over shared tables and
/// the suite shares one database, so parallel runtimes would delete each
/// other's rows mid-assertion.
#[tokio::test]
async fn request_activity_is_counted_flushed_and_reported() {
    the_middleware_records_traffic_and_ignores_the_console().await;
    flushing_twice_accumulates().await;
    the_flush_prunes_what_has_aged_out().await;
    the_activity_endpoint_is_gated_and_shaped().await;
    the_cohorts_split_new_from_returning().await;
    the_live_activity_section_is_served_from_memory().await;
}
