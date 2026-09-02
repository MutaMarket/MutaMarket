//! Behavior tests for the moderator contract review (legacy
//! `ModeratorContractController`): the public page data with its random
//! reviewable pick, the type and needs-training filters, and the
//! login-gated review action with its audit trail.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// Fixture abyssal type of the seeded modules.
const WEBIFIER_TYPE_ID: i64 = 47702;
const FORGE_REGION_ID: i64 = 10_000_002;

/// A synthetic abyssal type owned by this suite, so the needs-training
/// assertions control their own estimator statistics row.
const NEEDY_TYPE_ID: i64 = 990_008_100;

const MODULE_ID_BASE: i64 = 990_008_000;
const CONTRACT_ID_BASE: i64 = 990_008_500;
const ISSUER_ID: i64 = 990_008_901;

/// The one reviewable webifier contract; the only unknown-status single
/// abyssal item exchange of its type across the suites.
const REVIEWABLE: i64 = CONTRACT_ID_BASE;
/// Unknown status but carrying a non-abyssal item: never picked for
/// review, which makes it a safe target for the store mutation tests.
const MULTI_ITEM: i64 = CONTRACT_ID_BASE + 1;
/// The needy-type contract behind the needs-training assertions.
const NEEDY: i64 = CONTRACT_ID_BASE + 2;

/// The reviewer's session cookie.
static SEEDED: OnceCell<String> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static String {
    SEEDED.get_or_init(|| seed(pool)).await
}

async fn seed(pool: &PgPool) -> String {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables)
        .await
        .expect("seed reference tables");

    sqlx::query("delete from historic_contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean historic contracts");
    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from characters where id = $1")
        .bind(ISSUER_ID)
        .execute(pool)
        .await
        .expect("clean issuer");
    sqlx::query("delete from users where name = 'Moderator Reviewer'")
        .execute(pool)
        .await
        .expect("clean users");

    let reviewer_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Moderator Reviewer') returning id")
            .fetch_one(pool)
            .await
            .expect("create reviewer");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Review Seller', $2)")
        .bind(ISSUER_ID)
        .bind(reviewer_id)
        .execute(pool)
        .await
        .expect("create issuer");

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region");

    sqlx::query(
        "insert into types (id, name, published) values ($1, 'Review Needy Web', true)
         on conflict (id) do update set name = excluded.name, published = true",
    )
    .bind(NEEDY_TYPE_ID)
    .execute(pool)
    .await
    .expect("seed needy type");
    sqlx::query(
        "insert into estimator_statistics (type_id, name, data_count)
         values ($1, 'Review Needy Web', 10)
         on conflict (type_id) do update set data_count = 10",
    )
    .bind(NEEDY_TYPE_ID)
    .execute(pool)
    .await
    .expect("seed estimator statistic");

    for (offset, type_id) in [(0_i64, WEBIFIER_TYPE_ID), (1, NEEDY_TYPE_ID)] {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(MODULE_ID_BASE + offset)
            .bind(type_id)
            .execute(pool)
            .await
            .expect("create module");
    }

    // (contract, module, type, non-abyssal count): the reviewable pick,
    // the multi-item mutation target, and the needy-type pick.
    for (contract_id, module_offset, type_id, non_abyssal) in [
        (REVIEWABLE, 0_i64, WEBIFIER_TYPE_ID, 0_i32),
        (MULTI_ITEM, 0, WEBIFIER_TYPE_ID, 1),
        (NEEDY, 1, NEEDY_TYPE_ID, 0),
    ] {
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, date_issued, date_expired,
                  price, unified_price, abyssal_modules_count, non_abyssal_modules_count)
             values ($1, 'unknown', $2, $3, 'item_exchange', now() - interval '10 days',
                     now() - interval '3 days', 300000000, 300000000, 1, $4)",
        )
        .bind(contract_id)
        .bind(FORGE_REGION_ID)
        .bind(ISSUER_ID)
        .bind(non_abyssal)
        .execute(pool)
        .await
        .expect("create historic contract");
        sqlx::query(
            "insert into historic_contract_items
                 (historic_contract_id, record_id, type_id, item_id)
             values ($1, 1, $2, $3)",
        )
        .bind(contract_id)
        .bind(type_id)
        .bind(MODULE_ID_BASE + module_offset)
        .execute(pool)
        .await
        .expect("create historic contract item");
    }

    create_session(pool, reviewer_id, Some(ISSUER_ID))
        .await
        .expect("session")
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("valid request"),
        )
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

async fn post_review(
    app: &axum::Router,
    contract_id: i64,
    session: Option<&str>,
    body: serde_json::Value,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(format!("/moderator/contracts/{contract_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::REFERER, "/moderator/contracts/type/47702");
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    app.clone()
        .oneshot(
            builder
                .body(Body::from(body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("infallible")
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

#[tokio::test]
async fn page_serves_a_reviewable_contract_publicly() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // No session required: the legacy page route carries no middleware.
    let uri = format!("/api/moderator/contracts/type/{WEBIFIER_TYPE_ID}");
    let (status, body) = get_json(&app, &uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["contract", "search"]);

    // Only the unknown-status single-abyssal exchange qualifies: the
    // multi-item contract of the same type is never picked.
    let contract = &body["contract"];
    assert_eq!(contract["id"].as_i64(), Some(REVIEWABLE));
    assert_eq!(
        sorted_keys(contract),
        [
            "abyssal_modules_count",
            "asking_for_items",
            "date_expired",
            "date_issued",
            "id",
            "issuer",
            "modules",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "status",
            "type",
        ],
        "guests see the historic resource without the admin flag",
    );
    assert_eq!(contract["status"].as_str(), Some("unknown"));
    assert_eq!(
        sorted_keys(&contract["issuer"]),
        [
            "corporation_id",
            "description",
            "has_premium",
            "id",
            "name",
            "slug"
        ],
    );
    let modules = contract["modules"].as_array().expect("modules loaded");
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0]["id"].as_i64(), Some(MODULE_ID_BASE));
    crate::common::assert_default_module_keys(&modules[0], false, &[]);

    // The resolved filter echo.
    assert_eq!(sorted_keys(&body["search"]), ["needs_training", "type"]);
    assert_eq!(
        body["search"]["type"]["id"].as_i64(),
        Some(WEBIFIER_TYPE_ID)
    );
    assert!(body["search"]["needs_training"].is_null());

    // An unknown type keeps the legacy 404 message.
    let (status, body) = get_json(&app, "/api/moderator/contracts/type/not-a-type").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"].as_str(),
        Some("Please provide a valid type.")
    );

    // The unfiltered page answers publicly too.
    let (status, body) = get_json(&app, "/api/moderator/contracts").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["contract", "search"]);
    assert!(body["search"]["type"].is_null());
}

#[tokio::test]
async fn needs_training_filters_by_estimator_sample_count() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Ten samples sit under the default minimum of fifty.
    let base = format!("/api/moderator/contracts/type/{NEEDY_TYPE_ID}");
    let (_, body) = get_json(&app, &format!("{base}/needs-training")).await;
    assert_eq!(body["contract"]["id"].as_i64(), Some(NEEDY));
    assert_eq!(body["search"]["needs_training"].as_i64(), Some(50));

    // A numeric argument replaces the minimum; ten is not under five.
    let (_, body) = get_json(&app, &format!("{base}/needs-training/5")).await;
    assert!(body["contract"].is_null());
    assert_eq!(body["search"]["needs_training"].as_i64(), Some(5));

    // A non-numeric argument falls back to the default, the legacy
    // is_numeric ternary.
    let (_, body) = get_json(&app, &format!("{base}/needs-training/soon")).await;
    assert_eq!(body["contract"]["id"].as_i64(), Some(NEEDY));
    assert_eq!(body["search"]["needs_training"].as_i64(), Some(50));

    // Rust's float parser accepts nan/inf where PHP's is_numeric does
    // not; both fall back to the default too.
    for non_numeric in ["nan", "inf"] {
        let (_, body) = get_json(&app, &format!("{base}/needs-training/{non_numeric}")).await;
        assert_eq!(body["search"]["needs_training"].as_i64(), Some(50));
    }

    // A well-trained type no longer needs reviews.
    sqlx::query("update estimator_statistics set data_count = 100 where type_id = $1")
        .bind(NEEDY_TYPE_ID)
        .execute(&pool)
        .await
        .expect("raise data count");
    let (_, body) = get_json(&app, &format!("{base}/needs-training")).await;
    assert!(body["contract"].is_null());
    sqlx::query("update estimator_statistics set data_count = 10 where type_id = $1")
        .bind(NEEDY_TYPE_ID)
        .execute(&pool)
        .await
        .expect("restore data count");
}

#[tokio::test]
async fn review_updates_status_with_an_audit_row() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let session = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests are redirected to the login page.
    let response = post_review(
        &app,
        MULTI_ITEM,
        None,
        serde_json::json!({"status": "completed"}),
    )
    .await;
    assert!(response.status().is_redirection());
    assert_eq!(response.headers()[header::LOCATION], "/login");

    // The legacy validation texts: required and enum rule.
    let response = post_review(&app, MULTI_ITEM, Some(session), serde_json::json!({})).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: serde_json::Value = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json");
    assert_eq!(
        body["message"].as_str(),
        Some("The given data was invalid.")
    );
    assert_eq!(
        body["errors"]["status"][0].as_str(),
        Some("The status field is required.")
    );

    let response = post_review(
        &app,
        MULTI_ITEM,
        Some(session),
        serde_json::json!({"status": "sold"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: serde_json::Value = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json");
    assert_eq!(
        body["errors"]["status"][0].as_str(),
        Some("The selected status is invalid.")
    );

    // Route model binding: unknown contracts are a 404, and Laravel
    // resolves the binding before the FormRequest validates, so even an
    // invalid payload answers 404, not 422.
    let response = post_review(
        &app,
        CONTRACT_ID_BASE + 99,
        Some(session),
        serde_json::json!({"status": "completed"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = post_review(
        &app,
        CONTRACT_ID_BASE + 99,
        Some(session),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // A valid review updates the contract, records the audit row and
    // redirects back to the referring page.
    let response = post_review(
        &app,
        MULTI_ITEM,
        Some(session),
        serde_json::json!({"status": "completed"}),
    )
    .await;
    assert!(response.status().is_redirection());
    assert_eq!(
        response.headers()[header::LOCATION],
        "/moderator/contracts/type/47702"
    );

    let status: String = sqlx::query_scalar("select status from historic_contracts where id = $1")
        .bind(MULTI_ITEM)
        .fetch_one(&pool)
        .await
        .expect("status");
    assert_eq!(status, "completed");
    let audit: (String, Option<String>) = sqlx::query_as(
        "select new_status, previous_status from contract_review_history
         where historic_contract_id = $1 order by id desc limit 1",
    )
    .bind(MULTI_ITEM)
    .fetch_one(&pool)
    .await
    .expect("audit row");
    assert_eq!(audit.0, "completed");
    assert_eq!(audit.1.as_deref(), Some("unknown"));

    // A second review hits the legacy already-reviewed guard.
    let response = post_review(
        &app,
        MULTI_ITEM,
        Some(session),
        serde_json::json!({"status": "failed"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json");
    assert_eq!(
        body["message"].as_str(),
        Some("The contract has already been reviewed.")
    );
}
