//! Behavior tests for `GET /api/premium/page`, the legacy
//! `PremiumController::index` page props: the nine newest modules with a
//! latest contract, serialized as guest module cards.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use sqlx::PgPool;
use tower::ServiceExt;

/// Fixture abyssal type of the seeded modules.
const WEBIFIER_TYPE_ID: i64 = 47702;
const FORGE_REGION_ID: i64 = 10_000_002;

/// High above every real item id (they sit in the low trillions), so the
/// seeded modules are always the newest by id.
const MODULE_ID_BASE: i64 = 9_100_000_000_000_000;
const CONTRACT_ID_BASE: i64 = 990_005_500;
const ISSUER_ID: i64 = 990_005_901;

async fn seed(pool: &PgPool) {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables).await.expect("seed reference tables");

    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean contracts");
    sqlx::query(
        "insert into characters (id, name) values ($1, 'Premium Sampler')
         on conflict (id) do update set name = excluded.name",
    )
    .bind(ISSUER_ID)
    .execute(pool)
    .await
    .expect("issuer");
    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region");

    // Three for-sale modules and one without a contract (excluded by the
    // legacy hasLatestContract scope).
    for offset in 0..4i64 {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(MODULE_ID_BASE + offset)
            .bind(WEBIFIER_TYPE_ID)
            .execute(pool)
            .await
            .expect("create module");
        if offset == 3 {
            continue;
        }
        sqlx::query(
            "insert into contracts
                 (id, region_id, issuer_id, type, date_issued, date_expired, price,
                  unified_price, abyssal_modules_count, plex_count)
             values ($1, $2, $3, 'item_exchange', now() - interval '1 day',
                     now() + interval '13 days', 300000000, 300000000, 1, 0)",
        )
        .bind(CONTRACT_ID_BASE + offset)
        .bind(FORGE_REGION_ID)
        .bind(ISSUER_ID)
        .execute(pool)
        .await
        .expect("create contract");
        sqlx::query("update modules set latest_contract_id = $2 where id = $1")
            .bind(MODULE_ID_BASE + offset)
            .bind(CONTRACT_ID_BASE + offset)
            .execute(pool)
            .await
            .expect("link contract");
    }
}

fn sorted_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
    keys.sort_unstable();
    keys
}

#[tokio::test]
async fn the_premium_page_serves_the_newest_for_sale_modules() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    seed(&pool).await;
    let app = mutamarket::server::test_router().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/premium/page")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");

    assert_eq!(sorted_keys(&body), ["sample_modules"]);
    let modules = body["sample_modules"].as_array().expect("modules");
    assert!(modules.len() <= 9, "the legacy limit of nine");

    // Newest first: our three for-sale modules lead; the contract-less
    // fourth never appears.
    let ids: Vec<i64> =
        modules.iter().map(|module| module["id"].as_i64().expect("id")).collect();
    assert_eq!(
        &ids[..3],
        [MODULE_ID_BASE + 2, MODULE_ID_BASE + 1, MODULE_ID_BASE],
        "ids: {ids:?}",
    );
    assert!(!ids.contains(&(MODULE_ID_BASE + 3)), "contract-less modules are excluded");

    // Guest card key set (the legacy ModuleResource with default
    // relations), and every sample carries its contract.
    for module in modules {
        assert_eq!(
            sorted_keys(module),
            [
                "average_fraction",
                "contract",
                "creator",
                "estimated_value",
                "estimated_value_updated_at",
                "id",
                "mutaplasmid",
                "mutated_attributes",
                "public_asset",
                "slug",
                "source_type",
                "type",
            ],
        );
        assert!(!module["contract"].is_null(), "every sample module is for sale");
    }
    assert_eq!(modules[0]["contract"]["issuer"]["name"], serde_json::json!("Premium Sampler"));

    // Teardown: the seeded modules are for-sale rows of a fixture type
    // and would leak into the search suite's exact result lists (the
    // setup cleanup above still covers a panicked run on the next pass).
    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(&pool)
        .await
        .expect("teardown modules");
    sqlx::query("delete from contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(&pool)
        .await
        .expect("teardown contracts");
}
