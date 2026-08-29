//! Behavior tests for the in-process module value estimator: the legacy
//! `EstimatorQueryResource` feature engineering against a stored native
//! model, the store/clear/skip paths of `EstimateModuleValue`, the batch
//! pass of `app:estimate-values`, the authenticated `POST /estimate/{module}`
//! endpoint, and the statistics seed + `/api/estimator-statistics` shape.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::forest::{Dataset, Forest};
use mutamarket::estimator::{self, Estimator};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// Synthetic ids owned by this suite only, so its statistics rows,
/// modules and models never collide with the fixture-driven suites.
const MODELED_TYPE: i64 = 990_000_101;
const UNTRAINED_TYPE: i64 = 990_000_102;
const MODELLESS_TYPE: i64 = 990_000_105;
const MISMATCH_TYPE: i64 = 990_000_106;
const BATCH_TYPE: i64 = 990_000_103;
const IDLE_TYPE: i64 = 990_000_104;
const SOURCE_TYPE: i64 = 990_000_111;

const ATTRIBUTE_MUTATED: i64 = 990_000_121;
const ATTRIBUTE_FALLBACK: i64 = 990_000_122;
const ATTRIBUTE_DERIVED: i64 = 990_000_123;

const PAYLOAD_MODULE: i64 = 990_100_001;
const UNTRAINED_MODULE: i64 = 990_100_002;
const MODELLESS_MODULE: i64 = 990_100_003;
const MISMATCH_MODULE: i64 = 990_100_004;
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
            seed_reference(&pool, &tables)
                .await
                .expect("seed reference tables");
            estimator::seed::seed_estimator_attributes(&pool)
                .await
                .expect("seed estimator attributes");
        })
        .await;

    pool
}

/// The full production router; the estimate endpoint needs no reference
/// data, so the in-memory tables stay empty.
fn app(pool: &PgPool) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new(
            "http://127.0.0.1:9",
            "client-id",
            "client-secret",
            "http://test/eve/callback",
        ),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(ReferenceData::default()),
        None,
    )
}

/// A logged-in session cookie for a fresh user.
async fn session_cookie(pool: &PgPool) -> String {
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Estimator Tester') returning id")
            .fetch_one(pool)
            .await
            .expect("create user");
    let token = create_session(pool, user_id, None)
        .await
        .expect("create session");

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

/// Stores a native model for the type, fitted on the given single-feature
/// rows (feature names beyond the first get constant zeros appended).
async fn seed_model(pool: &PgPool, type_id: i64, feature_names: &[&str], rows: &[(Vec<f32>, f32)]) {
    let names: Vec<String> = feature_names
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    let data = Dataset {
        n_features: names.len(),
        features: rows
            .iter()
            .flat_map(|(features, _)| features.iter().copied())
            .collect(),
        targets: rows.iter().map(|(_, target)| *target).collect(),
    };
    let forest = Forest::fit(&data, names, estimator::forest::RANDOM_STATE);

    sqlx::query(
        "insert into estimator_models (type_id, feature_names, model, trained_at)
         values ($1, $2, $3, now())
         on conflict (type_id) do update
         set feature_names = excluded.feature_names, model = excluded.model,
             trained_at = excluded.trained_at",
    )
    .bind(type_id)
    .bind(serde_json::to_value(&forest.feature_names).expect("names"))
    .bind(forest.to_bytes())
    .execute(pool)
    .await
    .expect("seed model");
}

/// The synthetic feature attributes and their estimator registration for
/// a type: a mutated one, a source-fallback one and a derived one that
/// must never become a feature.
async fn seed_feature_attributes(pool: &PgPool, type_id: i64) {
    for (id, name, derived) in [
        (ATTRIBUTE_MUTATED, "estimatorSuiteMutated", false),
        (ATTRIBUTE_FALLBACK, "estimatorSuiteFallback", false),
        (ATTRIBUTE_DERIVED, "estimatorSuiteDerived", true),
    ] {
        sqlx::query(
            "insert into attributes (id, name, derived) values ($1, $2, $3)
             on conflict (id) do update set name = excluded.name, derived = excluded.derived",
        )
        .bind(id)
        .bind(name)
        .bind(derived)
        .execute(pool)
        .await
        .expect("seed attribute");
    }

    sqlx::query("delete from estimator_attributes where type_id = $1")
        .bind(type_id)
        .execute(pool)
        .await
        .expect("clean estimator attributes");
    for attribute in [ATTRIBUTE_MUTATED, ATTRIBUTE_FALLBACK, ATTRIBUTE_DERIVED] {
        sqlx::query("insert into estimator_attributes (type_id, attribute_id) values ($1, $2)")
            .bind(type_id)
            .bind(attribute)
            .execute(pool)
            .await
            .expect("seed estimator attribute");
    }

    seed_type(pool, SOURCE_TYPE, "Estimator Suite Source").await;
    sqlx::query(
        "insert into type_attributes (id, type_id, attribute_id, value)
         values ($1, $2, $3, 7.5)
         on conflict (type_id, attribute_id) do update set value = excluded.value",
    )
    .bind(990_000_131_i64)
    .bind(SOURCE_TYPE)
    .bind(ATTRIBUTE_FALLBACK)
    .execute(pool)
    .await
    .expect("seed source attribute");
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
async fn estimate_endpoint_predicts_from_the_legacy_features_and_stores_the_value() {
    let pool = setup().await;

    seed_type(&pool, MODELED_TYPE, "Estimator Suite Modeled").await;
    seed_feature_attributes(&pool, MODELED_TYPE).await;
    seed_statistic(&pool, MODELED_TYPE, "estimator_suite_modeled", Some(0.9)).await;

    // The model splits on the mutated feature: below 100 predicts
    // 1_000_000, above predicts 9_000_000. The fallback feature is
    // constant so the tree never splits on it. Feature order is
    // name-sorted like training: [Fallback, Mutated].
    let mut rows = Vec::new();
    for index in 0..20 {
        let mutated = if index % 2 == 0 { 50.0 } else { 150.0 };
        let price = if index % 2 == 0 {
            1_000_000.0
        } else {
            9_000_000.0
        };
        rows.push((vec![7.5_f32, mutated], price));
    }
    seed_model(
        &pool,
        MODELED_TYPE,
        &["estimatorSuiteFallback", "estimatorSuiteMutated"],
        &rows,
    )
    .await;

    seed_module(&pool, PAYLOAD_MODULE, MODELED_TYPE, Some(SOURCE_TYPE)).await;
    seed_mutated_attribute(&pool, PAYLOAD_MODULE, ATTRIBUTE_MUTATED, 150.0).await;

    let app = app(&pool);
    let cookie = session_cookie(&pool).await;

    let response = app
        .clone()
        .oneshot(post_estimate(
            PAYLOAD_MODULE,
            Some(&cookie),
            Some("/modules/some-module-1"),
        ))
        .await
        .expect("infallible");

    // back() to the referer (302 in Laravel; axum's Redirect uses 303).
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/modules/some-module-1");

    // The mutated value routes to the high leaf; the derived attribute is
    // no feature and the fallback came from the source type (a wrong
    // fallback would still predict 9M here, so also assert the low side
    // through the module's twin below).
    let (value, updated_at) = estimate_columns(&pool, PAYLOAD_MODULE).await;
    assert_eq!(value, Some(9_000_000.0));
    assert!(updated_at.is_some());

    // Re-seed the mutated value onto the low side: the prediction follows.
    seed_mutated_attribute(&pool, PAYLOAD_MODULE, ATTRIBUTE_MUTATED, 50.0).await;
    let stored = estimator::estimate_module_value(&pool, &Estimator::new(), PAYLOAD_MODULE)
        .await
        .expect("estimate runs");
    assert!(stored);
    assert_eq!(
        estimate_columns(&pool, PAYLOAD_MODULE).await.0,
        Some(1_000_000.0)
    );
}

#[tokio::test]
async fn estimate_clears_the_value_when_no_model_is_trained() {
    let pool = setup().await;

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

    let app = app(&pool);
    let cookie = session_cookie(&pool).await;

    // Without a Referer the redirect falls back to home.
    let response = app
        .clone()
        .oneshot(post_estimate(UNTRAINED_MODULE, Some(&cookie), None))
        .await
        .expect("infallible");
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/");

    // The stored estimate is cleared and the timestamp still advances.
    let (value, updated_at) = estimate_columns(&pool, UNTRAINED_MODULE).await;
    assert_eq!(value, None);
    assert!(updated_at.is_some());
}

#[tokio::test]
async fn missing_or_mismatched_models_leave_the_estimate_untouched() {
    let pool = setup().await;
    let estimator = Estimator::new();

    // A trained statistic without a stored model (the legacy AI server
    // 404ing on a missing artifact): nothing is stored.
    seed_type(&pool, MODELLESS_TYPE, "Estimator Test Modelless").await;
    seed_statistic(&pool, MODELLESS_TYPE, "estimator_test_modelless", Some(0.5)).await;
    sqlx::query("delete from estimator_models where type_id = $1")
        .bind(MODELLESS_TYPE)
        .execute(&pool)
        .await
        .expect("clean model");
    seed_module(&pool, MODELLESS_MODULE, MODELLESS_TYPE, None).await;
    sqlx::query(
        "update modules
         set estimated_value = 700000000, estimated_value_updated_at = now() - interval '1 day'
         where id = $1",
    )
    .bind(MODELLESS_MODULE)
    .execute(&pool)
    .await
    .expect("preset estimate");
    let before = estimate_columns(&pool, MODELLESS_MODULE).await;

    let stored = estimator::estimate_module_value(&pool, &estimator, MODELLESS_MODULE)
        .await
        .expect("estimate runs");
    assert!(!stored);
    assert_eq!(estimate_columns(&pool, MODELLESS_MODULE).await, before);

    // A stored model whose features no longer match the type's estimator
    // attributes (the legacy query server's 422): nothing is stored.
    seed_type(&pool, MISMATCH_TYPE, "Estimator Test Mismatch").await;
    seed_statistic(&pool, MISMATCH_TYPE, "estimator_test_mismatch", Some(0.5)).await;
    sqlx::query("delete from estimator_attributes where type_id = $1")
        .bind(MISMATCH_TYPE)
        .execute(&pool)
        .await
        .expect("clean estimator attributes");
    seed_model(
        &pool,
        MISMATCH_TYPE,
        &["estimatorSuiteMutated"],
        &[
            (vec![1.0], 5.0),
            (vec![2.0], 6.0),
            (vec![3.0], 7.0),
            (vec![4.0], 8.0),
        ],
    )
    .await;
    seed_module(&pool, MISMATCH_MODULE, MISMATCH_TYPE, None).await;
    let before = estimate_columns(&pool, MISMATCH_MODULE).await;

    let stored = estimator::estimate_module_value(&pool, &estimator, MISMATCH_MODULE)
        .await
        .expect("estimate runs");
    assert!(!stored);
    assert_eq!(estimate_columns(&pool, MISMATCH_MODULE).await, before);

    // A missing module is a no-op false.
    let stored = estimator::estimate_module_value(&pool, &estimator, 990_199_999)
        .await
        .expect("estimate runs");
    assert!(!stored);
}

#[tokio::test]
async fn estimate_passes_pick_stalest_trained_modules_first() {
    let pool = setup().await;
    let estimator = Estimator::new();

    seed_type(&pool, BATCH_TYPE, "Estimator Test Batch").await;
    seed_type(&pool, IDLE_TYPE, "Estimator Test Idle").await;
    seed_statistic(&pool, BATCH_TYPE, "Estimator Test Batch", Some(0.7)).await;
    seed_statistic(&pool, IDLE_TYPE, "Estimator Test Idle", None).await;

    // A featureless model (no estimator attributes): every prediction is
    // the constant mean, here 42.
    sqlx::query("delete from estimator_attributes where type_id = $1")
        .bind(BATCH_TYPE)
        .execute(&pool)
        .await
        .expect("clean estimator attributes");
    seed_model(&pool, BATCH_TYPE, &[], &[(vec![], 42.0), (vec![], 42.0)]).await;

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
    sqlx::query(
        "update modules set estimated_value_updated_at = now() - interval '2 days' where id = $1",
    )
    .bind(BATCH_MODULE_OLD)
    .execute(&pool)
    .await
    .expect("age module");
    sqlx::query(
        "update modules set estimated_value_updated_at = now() - interval '1 day' where id = $1",
    )
    .bind(BATCH_MODULE_RECENT)
    .execute(&pool)
    .await
    .expect("age module");

    // count=2: the never-estimated module first (nulls first), then the
    // oldest; the recent one misses the cut and the untrained type is
    // excluded entirely.
    let run = estimator::estimate_values(&pool, &estimator, 2, Some("Estimator Test Batch"))
        .await
        .expect("estimate pass");
    assert_eq!((run.attempted, run.updated), (2, 2));

    assert_eq!(
        estimate_columns(&pool, BATCH_MODULE_NEVER).await.0,
        Some(42.0)
    );
    assert_eq!(
        estimate_columns(&pool, BATCH_MODULE_OLD).await.0,
        Some(42.0)
    );
    assert_eq!(estimate_columns(&pool, BATCH_MODULE_RECENT).await.0, None);
    assert_eq!(estimate_columns(&pool, IDLE_MODULE).await, (None, None));

    // A second, uncapped pass reaches the remaining module.
    let run = estimator::estimate_values(&pool, &estimator, 10, Some("Estimator Test Batch"))
        .await
        .expect("estimate pass");
    assert_eq!((run.attempted, run.updated), (3, 3));
    assert_eq!(
        estimate_columns(&pool, BATCH_MODULE_RECENT).await.0,
        Some(42.0)
    );
    assert_eq!(estimate_columns(&pool, IDLE_MODULE).await, (None, None));

    // An unknown type fragment fails like the legacy firstOrFail.
    let missing =
        estimator::estimate_values(&pool, &estimator, 1, Some("No Such Abyssal Type")).await;
    assert!(missing.is_err());
}

#[tokio::test]
async fn estimate_endpoint_guards_sessions_and_unknown_modules() {
    let pool = setup().await;
    let app = app(&pool);

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

    let app = app(&pool);

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
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
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
    let mut keys: Vec<&str> = row
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
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
