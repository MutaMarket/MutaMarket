//! Behavior tests for `GET /api/historic-sales-cards[/{query}]`, the
//! premium historic-sales browser (legacy `HistoricSaleController`):
//! the premium gate, the newest-sale-first default, the historic price
//! sort and the single-bound-is-a-maximum price filter.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
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

const MODULE_ID_BASE: i64 = 990_004_000;
const CONTRACT_ID_BASE: i64 = 990_004_500;
const ISSUER_ID: i64 = 990_004_901;

/// Three sales: (module offset, price, issued days ago).
const SALES: [(i64, f64, i32); 3] = [
    (0, 500_000_000.0, 3),
    (1, 100_000_000.0, 1),
    (2, 900_000_000.0, 2),
];

static SEEDED: OnceCell<(String, String)> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static (String, String) {
    SEEDED.get_or_init(|| seed(pool)).await
}

async fn seed(pool: &PgPool) -> (String, String) {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables)
        .await
        .expect("seed reference tables");

    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from historic_contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean historic contracts");
    sqlx::query("delete from characters where id = $1")
        .bind(ISSUER_ID)
        .execute(pool)
        .await
        .expect("clean issuer");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Historic Premium", "Historic Pleb"])
        .execute(pool)
        .await
        .expect("clean users");

    let premium_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Historic Premium') returning id")
            .fetch_one(pool)
            .await
            .expect("create premium user");
    let pleb_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Historic Pleb') returning id")
            .fetch_one(pool)
            .await
            .expect("create pleb");
    sqlx::query(
        "insert into characters (id, name, user_id, premium_paid_until)
         values ($1, 'Historic Seller', $2, now() + interval '30 days')",
    )
    .bind(ISSUER_ID)
    .bind(premium_id)
    .execute(pool)
    .await
    .expect("create premium character");

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region");

    for (offset, price, days_ago) in SALES {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(MODULE_ID_BASE + offset)
            .bind(WEBIFIER_TYPE_ID)
            .execute(pool)
            .await
            .expect("create module");
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, date_issued, price, unified_price)
             values ($1, 'expired', $2, $3, 'item_exchange',
                     now() - make_interval(days => $4), $5, $5)",
        )
        .bind(CONTRACT_ID_BASE + offset)
        .bind(FORGE_REGION_ID)
        .bind(ISSUER_ID)
        .bind(days_ago)
        .bind(price)
        .execute(pool)
        .await
        .expect("create historic contract");
        sqlx::query(
            "insert into training_modules (module_id, historic_contract_id, issued_at)
             values ($1, $2, now() - make_interval(days => $3))
             on conflict (module_id) do update
             set historic_contract_id = excluded.historic_contract_id,
                 issued_at = excluded.issued_at",
        )
        .bind(MODULE_ID_BASE + offset)
        .bind(CONTRACT_ID_BASE + offset)
        .bind(days_ago)
        .execute(pool)
        .await
        .expect("create training row");
    }

    let premium = create_session(pool, premium_id, Some(ISSUER_ID))
        .await
        .expect("session");
    let pleb = create_session(pool, pleb_id, None).await.expect("session");
    (premium, pleb)
}

async fn get_json(
    app: &axum::Router,
    uri: &str,
    session: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().uri(uri);
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

/// The suite's module ids in response order.
fn seeded_ids(body: &serde_json::Value) -> Vec<i64> {
    body.as_array()
        .expect("a bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .filter(|id| (MODULE_ID_BASE..MODULE_ID_BASE + 100).contains(id))
        .collect()
}

#[tokio::test]
async fn historic_sales_gate_sort_and_price_semantics() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (premium, pleb) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // The legacy PremiumMiddleware: guests 401, non-premium 403.
    let (status, body) = get_json(&app, "/api/historic-sales-cards", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));
    let (status, body) = get_json(&app, "/api/historic-sales-cards", Some(pleb)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"].as_str(), Some("Premium required."));

    // Newest sale first (the legacy orderByTrainingIssuedAt default).
    let base = format!("/api/historic-sales-cards/type/{WEBIFIER_TYPE_ID}");
    let (status, body) = get_json(&app, &base, Some(premium)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        seeded_ids(&body),
        vec![MODULE_ID_BASE + 1, MODULE_ID_BASE + 2, MODULE_ID_BASE],
    );

    // Every card carries the historic sale with the exact resource keys.
    let card = body
        .as_array()
        .expect("cards")
        .iter()
        .find(|module| module["id"].as_i64() == Some(MODULE_ID_BASE + 1))
        .expect("seeded card");
    crate::common::assert_default_module_keys(card, true, &["training_module"]);
    let training = card["training_module"]
        .as_object()
        .expect("training loaded");
    let mut keys: Vec<&str> = training.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["contract_id", "sold_at", "sold_for"]);
    assert_eq!(training["sold_for"].as_f64(), Some(100_000_000.0));
    assert_eq!(training["contract_id"].as_i64(), Some(CONTRACT_ID_BASE + 1));

    // Price sorting over the recorded sale price.
    let (_, asc) = get_json(&app, &format!("{base}/sort/price/asc"), Some(premium)).await;
    assert_eq!(
        seeded_ids(&asc),
        vec![MODULE_ID_BASE + 1, MODULE_ID_BASE, MODULE_ID_BASE + 2],
    );
    let (_, desc) = get_json(&app, &format!("{base}/sort/price/desc"), Some(premium)).await;
    assert_eq!(
        seeded_ids(&desc),
        vec![MODULE_ID_BASE + 2, MODULE_ID_BASE, MODULE_ID_BASE + 1],
    );

    // The legacy whereHistoricPrice quirks: a single bound is a maximum,
    // a range is inclusive, a zero lower bound disables the filter.
    let (_, max_bound) = get_json(
        &app,
        &format!("{base}/contract-price/500000000"),
        Some(premium),
    )
    .await;
    assert_eq!(
        seeded_ids(&max_bound),
        vec![MODULE_ID_BASE + 1, MODULE_ID_BASE],
    );
    let (_, range) = get_json(
        &app,
        &format!("{base}/contract-price/400000000-1000000000"),
        Some(premium),
    )
    .await;
    assert_eq!(seeded_ids(&range), vec![MODULE_ID_BASE + 2, MODULE_ID_BASE]);
    let (_, zero) = get_json(&app, &format!("{base}/contract-price/0-100"), Some(premium)).await;
    assert_eq!(
        seeded_ids(&zero).len(),
        3,
        "zero lower bound disables the filter"
    );

    // The regular browse cards never carry the training key.
    let (_, browse) = get_json(&app, "/api/module-cards?unlisted=true", None).await;
    for module in browse.as_array().expect("cards") {
        assert!(
            module.get("training_module").is_none(),
            "training stays absent outside the historic page",
        );
    }
}
