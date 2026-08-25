//! Behavior tests for the module value estimator against a mock AI server:
//! the exact feature payload of the legacy `EstimatorQueryResource`, the
//! store/clear/skip paths of `EstimateModuleValue`, the batch pass of
//! `app:estimate-values`, the authenticated `POST /estimate/{module}`
//! endpoint, and the statistics seed + `/api/estimator-statistics` shape.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::{self, EstimatorClient};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// 5MN Abyssal Microwarpdrive — a type with seeded estimator attributes.
const ABYSSAL_5MN_MWD: i64 = 47740;

/// 5MN Microwarpdrive I, an input source type of the 5MN mutaplasmids.
const SOURCE_5MN_MWD_I: i64 = 434;

/// Dogma attribute ids of the mutated values used in the payload test.
const CAPACITOR_NEED: i64 = 6;
const SPEED_FACTOR: i64 = 20;

/// Synthetic ids owned by this suite only, so its statistics rows and
/// modules never collide with the fixture-driven suites.
const UNTRAINED_TYPE: i64 = 990_000_101;
const FAILING_TYPE: i64 = 990_000_102;
const BATCH_TYPE: i64 = 990_000_103;
const IDLE_TYPE: i64 = 990_000_104;

const PAYLOAD_MODULE: i64 = 990_100_001;
const UNTRAINED_MODULE: i64 = 990_100_002;
const FAILING_MODULE: i64 = 990_100_003;
const BATCH_MODULE_OLD: i64 = 990_100_011;
const BATCH_MODULE_NEVER: i64 = 990_100_012;
const BATCH_MODULE_RECENT: i64 = 990_100_013;
const IDLE_MODULE: i64 = 990_100_014;

/// Reference data and the estimator attribute seed load once per binary;
/// the individual tests only touch rows keyed by their own ids.
async fn setup() -> PgPool {
    static SETUP: OnceCell<()> = OnceCell::const_new();

    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    SETUP
        .get_or_init(|| async {
            let tables = ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference"))
                .expect("dumps parse");
            seed_reference(&pool, &tables).await.expect("seed reference tables");
            estimator::seed::seed_estimator_attributes(&pool)
                .await
                .expect("seed estimator attributes");
        })
        .await;

    pool
}

/// Mock AI estimation server recording every `(model_name, raw_body)` and
/// answering with the given status (and value when successful).
async fn start_mock_ai(
    status: StatusCode,
    estimated_value: f64,
) -> (String, Arc<Mutex<Vec<(String, String)>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();

    let app = Router::new().route(
        "/estimate/{model}",
        post(move |AxumPath(model): AxumPath<String>, body: String| {
            let recorded = recorded.clone();
            async move {
                recorded.lock().expect("requests lock").push((model, body));
                if status.is_success() {
                    Json(json!({ "estimated_value": estimated_value })).into_response()
                } else {
                    status.into_response()
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock AI");
    let address = listener.local_addr().expect("mock AI address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock AI");
    });

    (format!("http://{address}"), requests)
}

/// The full production router against the given AI server. The estimate
/// endpoint needs no reference data, so the in-memory tables stay empty.
fn app(pool: &PgPool, ai_url: &str) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new("http://127.0.0.1:9", "client-id", "client-secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        EstimatorClient::new(ai_url),
        Arc::new(ReferenceData::default()),
        None,
    )
}

/// A logged-in session cookie for a fresh user.
async fn session_cookie(pool: &PgPool) -> String {
    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ('Estimator Tester') returning id")
        .fetch_one(pool)
        .await
        .expect("create user");
    let token = create_session(pool, user_id, None).await.expect("create session");

    format!("mm_session={token}")
}

async fn seed_type(pool: &PgPool, type_id: i64, name: &str) {
    sqlx::query(
        "insert into types (id, name, published) values ($1, $2, true)
         on conflict (id) do update set name = excluded.name",
    )
    .bind(type_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("seed type");
}

/// Upserts a module and resets its estimate columns.
async fn seed_module(pool: &PgPool, module_id: i64, type_id: i64, source_type_id: Option<i64>) {
    sqlx::query(
        "insert into modules (id, type_id, source_type_id)
         values ($1, $2, $3)
         on conflict (id) do update set
             type_id = excluded.type_id,
             source_type_id = excluded.source_type_id,
             estimated_value = null,
             estimated_value_updated_at = null",
    )
    .bind(module_id)
    .bind(type_id)
    .bind(source_type_id)
    .execute(pool)
    .await
    .expect("seed module");
}

async fn seed_statistic(pool: &PgPool, type_id: i64, name: &str, r2: Option<f64>) {
    sqlx::query(
        "insert into estimator_statistics (type_id, name, data_count, r2, data_statistics)
         values ($1, $2, 0, $3, '{}'::jsonb)
         on conflict (type_id) do update set name = excluded.name, r2 = excluded.r2",
    )
    .bind(type_id)
    .bind(name)
    .bind(r2)
    .execute(pool)
    .await
    .expect("seed statistic");
}

async fn seed_mutated_attribute(pool: &PgPool, module_id: i64, attribute_id: i64, value: f64) {
    sqlx::query(
        "insert into mutated_attributes
         (module_id, attribute_id, type_id, value, base_value, fraction, fraction_type,
          fraction_absolute, bar, is_virtual)
         select $1, $2, m.type_id, $3, 0, 0, 0, 0, 0, false from modules m where m.id = $1
         on conflict (module_id, attribute_id) do update set value = excluded.value",
    )
    .bind(module_id)
    .bind(attribute_id)
    .bind(value)
    .execute(pool)
    .await
    .expect("seed mutated attribute");
}

async fn estimate_columns(pool: &PgPool, module_id: i64) -> (Option<f64>, Option<String>) {
    sqlx::query_as(
        "select estimated_value, estimated_value_updated_at::text from modules where id = $1",
    )
    .bind(module_id)
    .fetch_one(pool)
    .await
    .expect("module estimate columns")
}

fn post_estimate(module: i64, cookie: Option<&str>, referer: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/estimate/{module}"));
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    if let Some(referer) = referer {
        builder = builder.header(header::REFERER, referer);
    }
    builder.body(Body::empty()).expect("request")
}

fn location(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn estimate_endpoint_sends_the_legacy_feature_payload_and_stores_the_value() {
    let pool = setup().await;
    let (ai_url, requests) = start_mock_ai(StatusCode::OK, 1_234_567.89).await;

    seed_module(&pool, PAYLOAD_MODULE, ABYSSAL_5MN_MWD, Some(SOURCE_5MN_MWD_I)).await;
    seed_mutated_attribute(&pool, PAYLOAD_MODULE, CAPACITOR_NEED, 43.2).await;
    seed_mutated_attribute(&pool, PAYLOAD_MODULE, SPEED_FACTOR, 512.5).await;
    seed_statistic(&pool, ABYSSAL_5MN_MWD, "5MN Abyssal Microwarpdrive", Some(0.9)).await;

    let app = app(&pool, &ai_url);
    let cookie = session_cookie(&pool).await;

    let response = app
        .clone()
        .oneshot(post_estimate(PAYLOAD_MODULE, Some(&cookie), Some("/modules/some-module-1")))
        .await
        .expect("infallible");

    // back() to the referer (302 in Laravel; axum's Redirect uses 303).
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/modules/some-module-1");

    // Exactly one AI call: the lowercased-underscored model name and the
    // EstimatorQueryResource payload — non-derived estimator attributes in
    // name order, mutated value ?? source type value ?? 0.
    let recorded = requests.lock().expect("requests lock").clone();
    assert_eq!(recorded.len(), 1);
    let (model, body) = &recorded[0];
    assert_eq!(model, "5mn_abyssal_microwarpdrive");
    assert_eq!(
        body,
        concat!(
            "{\"capacitorCapacityMultiplier\":0.75,\"capacitorNeed\":43.2,\"cpu\":25.0,",
            "\"overloadSpeedFactorBonus\":50.0,\"power\":15.0,\"signatureRadiusBonus\":500.0,",
            "\"speedFactor\":512.5}",
        ),
    );

    let (value, updated_at) = estimate_columns(&pool, PAYLOAD_MODULE).await;
    assert_eq!(value, Some(1_234_567.89));
    assert!(updated_at.is_some());
}

#[tokio::test]
async fn estimate_clears_the_value_when_no_model_is_trained() {
    let pool = setup().await;
    let (ai_url, requests) = start_mock_ai(StatusCode::OK, 1.0).await;

    seed_type(&pool, UNTRAINED_TYPE, "Estimator Test Untrained").await;
    sqlx::query("delete from estimator_statistics where type_id = $1")
        .bind(UNTRAINED_TYPE)
        .execute(&pool)
        .await
        .expect("clean statistic");
    seed_module(&pool, UNTRAINED_MODULE, UNTRAINED_TYPE, None).await;
    sqlx::query("update modules set estimated_value = 500000000 where id = $1")
        .bind(UNTRAINED_MODULE)
        .execute(&pool)
        .await
        .expect("preset estimate");

    let app = app(&pool, &ai_url);
    let cookie = session_cookie(&pool).await;

    // Without a Referer the redirect falls back to home.
    let response = app
        .clone()
        .oneshot(post_estimate(UNTRAINED_MODULE, Some(&cookie), None))
        .await
        .expect("infallible");
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/");

    // The stored estimate is cleared, the timestamp still advances, and no
    // AI call happens.
    let (value, updated_at) = estimate_columns(&pool, UNTRAINED_MODULE).await;
    assert_eq!(value, None);
    assert!(updated_at.is_some());
    assert!(requests.lock().expect("requests lock").is_empty());
}

#[tokio::test]
async fn failed_ai_responses_leave_the_estimate_untouched() {
    let pool = setup().await;

    seed_type(&pool, FAILING_TYPE, "Estimator Test Failing").await;
    seed_statistic(&pool, FAILING_TYPE, "Estimator Test Failing", Some(0.5)).await;
    seed_module(&pool, FAILING_MODULE, FAILING_TYPE, None).await;
    sqlx::query(
        "update modules
         set estimated_value = 700000000, estimated_value_updated_at = now() - interval '1 day'
         where id = $1",
    )
    .bind(FAILING_MODULE)
    .execute(&pool)
    .await
    .expect("preset estimate");
    let before = estimate_columns(&pool, FAILING_MODULE).await;

    // A 404 (no trained model artifact) fails without storing anything.
    let (ai_url, requests) = start_mock_ai(StatusCode::NOT_FOUND, 0.0).await;
    let stored = estimator::estimate_module_value(&pool, &EstimatorClient::new(&ai_url), FAILING_MODULE)
        .await
        .expect("estimate runs");
    assert!(!stored);
    assert_eq!(requests.lock().expect("requests lock").len(), 1);
    assert_eq!(estimate_columns(&pool, FAILING_MODULE).await, before);

    // An unreachable server behaves the same (the legacy swallowed
    // ConnectionException).
    let unreachable = EstimatorClient::new("http://127.0.0.1:1");
    let stored = estimator::estimate_module_value(&pool, &unreachable, FAILING_MODULE)
        .await
        .expect("estimate runs");
    assert!(!stored);
    assert_eq!(estimate_columns(&pool, FAILING_MODULE).await, before);

    // A missing module is a no-op false.
    let stored = estimator::estimate_module_value(&pool, &unreachable, 990_199_999)
        .await
        .expect("estimate runs");
    assert!(!stored);
}

#[tokio::test]
async fn estimate_passes_pick_stalest_trained_modules_first() {
    let pool = setup().await;
    let (ai_url, requests) = start_mock_ai(StatusCode::OK, 42.0).await;
    let client = EstimatorClient::new(&ai_url);

    seed_type(&pool, BATCH_TYPE, "Estimator Test Batch").await;
    seed_type(&pool, IDLE_TYPE, "Estimator Test Idle").await;
    seed_statistic(&pool, BATCH_TYPE, "Estimator Test Batch", Some(0.7)).await;
    seed_statistic(&pool, IDLE_TYPE, "Estimator Test Idle", None).await;

    // The pass resolves the type fragment against mutaplasmid output
    // types, so the synthetic type needs a synthetic mutaplasmid.
    sqlx::query(
        "insert into mutaplasmids (id, name, output_type_id)
         values ($1, 'Estimator Test Mutaplasmid', $1)
         on conflict (id) do nothing",
    )
    .bind(BATCH_TYPE)
    .execute(&pool)
    .await
    .expect("seed mutaplasmid");

    seed_module(&pool, BATCH_MODULE_OLD, BATCH_TYPE, None).await;
    seed_module(&pool, BATCH_MODULE_NEVER, BATCH_TYPE, None).await;
    seed_module(&pool, BATCH_MODULE_RECENT, BATCH_TYPE, None).await;
    seed_module(&pool, IDLE_MODULE, IDLE_TYPE, None).await;
    sqlx::query("update modules set estimated_value_updated_at = now() - interval '2 days' where id = $1")
        .bind(BATCH_MODULE_OLD)
        .execute(&pool)
        .await
        .expect("age module");
    sqlx::query("update modules set estimated_value_updated_at = now() - interval '1 day' where id = $1")
        .bind(BATCH_MODULE_RECENT)
        .execute(&pool)
        .await
        .expect("age module");

    // count=2: the never-estimated module first (nulls first), then the
    // oldest; the recent one misses the cut and the untrained type is
    // excluded entirely.
    let run = estimator::estimate_values(&pool, &client, 2, Some("Estimator Test Batch"))
        .await
        .expect("estimate pass");
    assert_eq!((run.attempted, run.updated), (2, 2));
    assert_eq!(requests.lock().expect("requests lock").len(), 2);

    assert_eq!(estimate_columns(&pool, BATCH_MODULE_NEVER).await.0, Some(42.0));
    assert_eq!(estimate_columns(&pool, BATCH_MODULE_OLD).await.0, Some(42.0));
    assert_eq!(estimate_columns(&pool, BATCH_MODULE_RECENT).await.0, None);
    assert_eq!(estimate_columns(&pool, IDLE_MODULE).await, (None, None));

    // A second, uncapped pass reaches the remaining module. The model name
    // derives from the type name.
    let run = estimator::estimate_values(&pool, &client, 10, Some("Estimator Test Batch"))
        .await
        .expect("estimate pass");
    assert_eq!((run.attempted, run.updated), (3, 3));
    assert_eq!(estimate_columns(&pool, BATCH_MODULE_RECENT).await.0, Some(42.0));
    assert_eq!(estimate_columns(&pool, IDLE_MODULE).await, (None, None));

    let recorded = requests.lock().expect("requests lock").clone();
    assert!(recorded.iter().all(|(model, body)| model == "estimator_test_batch" && body == "{}"));

    // An unknown type fragment fails like the legacy firstOrFail.
    let missing = estimator::estimate_values(&pool, &client, 1, Some("No Such Abyssal Type")).await;
    assert!(missing.is_err());
}

#[tokio::test]
async fn estimate_endpoint_guards_sessions_and_unknown_modules() {
    let pool = setup().await;
    let (ai_url, requests) = start_mock_ai(StatusCode::OK, 1.0).await;
    let app = app(&pool, &ai_url);

    // Guests are redirected to the login page.
    let response = app
        .clone()
        .oneshot(post_estimate(1, None, None))
        .await
        .expect("infallible");
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/login");

    // Unknown and non-numeric module ids 404 like the legacy implicit
    // route binding.
    let cookie = session_cookie(&pool).await;
    let response = app
        .clone()
        .oneshot(post_estimate(990_199_998, Some(&cookie), None))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/estimate/not-a-module")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    assert!(requests.lock().expect("requests lock").is_empty());
}

#[tokio::test]
async fn statistics_seed_and_endpoint_carry_the_legacy_shape() {
    let pool = setup().await;

    // Own the asserted rows: other suites (module_api's show-page test)
    // may have written synthetic statistics for the same types, and the
    // seed keeps existing rows (firstOrCreate).
    sqlx::query("delete from estimator_statistics")
        .execute(&pool)
        .await
        .expect("clean statistics");

    estimator::seed::seed_estimator_statistics(&pool)
        .await
        .expect("seed statistics");

    // Rerunning keeps existing rows (firstOrCreate).
    estimator::seed::seed_estimator_statistics(&pool)
        .await
        .expect("seed statistics again");

    let (ai_url, _requests) = start_mock_ai(StatusCode::OK, 1.0).await;
    let app = app(&pool, &ai_url);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/estimator-statistics")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");

    // The 50MN Abyssal Microwarpdrive row: untrained baseline with a zero
    // per meta group among its mutaplasmid input types.
    let row = body
        .as_array()
        .expect("statistics array")
        .iter()
        .find(|row| row["type_id"] == json!(47408))
        .expect("50MN MWD statistic seeded");

    // The exact legacy key set: EstimatorStatistic::all() serializes every
    // column.
    let mut keys: Vec<&str> = row.as_object().expect("object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "created_at",
            "data_count",
            "data_statistics",
            "id",
            "last_trained_at",
            "mae",
            "name",
            "nmae",
            "r2",
            "type_id",
            "updated_at",
        ],
    );

    assert_eq!(row["name"], json!("50MN Abyssal Microwarpdrive"));
    assert_eq!(row["data_count"], json!(0));
    assert_eq!(row["r2"], json!(null));
    assert_eq!(row["mae"], json!(null));
    assert_eq!(row["nmae"], json!(null));
    assert_eq!(row["last_trained_at"], json!(null));
    assert!(row["created_at"].is_string());
    assert!(row["updated_at"].is_string());
    assert_eq!(
        row["data_statistics"],
        json!({
            "Deadspace": 0,
            "Faction": 0,
            "Officer": 0,
            "Storyline": 0,
            "Tech I": 0,
            "Tech II": 0,
        }),
    );
}
