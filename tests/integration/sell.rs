//! Behavior tests for the sell page endpoints (the legacy
//! `SellController::index` + `Character::locations()`): the published
//! scope, the header stats, and the select-modules location list with
//! its publish round trip.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const SELLER_CHARACTER: i64 = 990_400_001;
const CONTAINER_ASSET_ITEM: i64 = 990_400_100;
const STATION: i64 = 60_003_760;

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables)
        .await
        .expect("seed");
    (pool, ReferenceData::from_tables(tables))
}

fn app(pool: &PgPool, reference: ReferenceData) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new(
            "http://127.0.0.1:9",
            "client",
            "secret",
            "http://test/eve/callback",
        ),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(reference),
        None,
    )
}

async fn send(
    app: &Router,
    method: &str,
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
    .expect("request");
    let response = app.clone().oneshot(request).await.expect("infallible");
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

#[tokio::test]
async fn the_sell_page_lists_published_modules_and_locations() {
    let (pool, reference) = setup().await;

    // A module owned by the seller, sitting inside a container asset.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[1];
    let module = &fixture.modules[0];
    process_module(
        &pool,
        &reference,
        &Estimator::new(),
        fixture.type_id,
        module.module_id,
        &DogmaItem {
            created_by: module.creator_id,
            source_type_id: module.source_type_id,
            mutator_type_id: module.mutaplasmid_id,
            dogma_attributes: common::fixture_dogma(module),
        },
    )
    .await
    .expect("process module");

    // Seller with a session; container + module assets (idempotent).
    sqlx::query("delete from public_assets where character_id = $1")
        .bind(SELLER_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean public assets");
    sqlx::query("delete from assets where character_id = $1")
        .bind(SELLER_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean assets");
    sqlx::query("delete from characters where id = $1")
        .bind(SELLER_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean character");
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Sell Tester') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Sell Tester', $2)")
        .bind(SELLER_CHARACTER)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("character");
    let session = create_session(&pool, user_id, Some(SELLER_CHARACTER))
        .await
        .expect("session");

    sqlx::query(
        "insert into stations (id, name, type_id, solarsystem_id) values ($1, $2, $3, $4)
         on conflict (id) do update set name = excluded.name",
    )
    .bind(STATION)
    .bind("Jita IV - Moon 4 - Caldari Navy Assembly Plant")
    .bind(52_678_i64)
    .bind(30_000_142_i64)
    .execute(&pool)
    .await
    .expect("seed station");

    let container_asset_id: i64 = sqlx::query_scalar(
        "insert into assets (character_id, item_id, type_id, name, location_id, location_flag,
                             location_type, quantity, is_abyssal)
         values ($1, $2, 3467, 'Sell Hangar Container', $3, 'Hangar', 'station', 1, false)
         returning id",
    )
    .bind(SELLER_CHARACTER)
    .bind(CONTAINER_ASSET_ITEM)
    .bind(STATION)
    .fetch_one(&pool)
    .await
    .expect("container asset");
    sqlx::query(
        "insert into assets (character_id, item_id, type_id, name, location_id, location_flag,
                             location_type, quantity, is_abyssal)
         values ($1, $2, $3, '', $4, 'Unlocked', 'item', 1, true)",
    )
    .bind(SELLER_CHARACTER)
    .bind(module.module_id)
    .bind(fixture.type_id)
    .bind(CONTAINER_ASSET_ITEM)
    .execute(&pool)
    .await
    .expect("module asset");

    let app = app(&pool, reference);

    // Guests are turned away everywhere.
    for path in ["/api/sell/page", "/api/sell/modules", "/api/sell/locations"] {
        let (status, body) = send(&app, "GET", path, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(body["message"], json!("Unauthenticated."));
    }

    // Before publishing: the container lists as private, nothing sells.
    let (status, body) = send(&app, "GET", "/api/sell/locations", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    let locations = body.as_array().expect("locations");
    assert_eq!(locations.len(), 1);
    let location = &locations[0];
    let mut keys: Vec<&str> = location
        .as_object()
        .expect("location")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "abyssal_count",
            "asset_id",
            "location_flag",
            "name",
            "public_asset_id",
            "station_name",
            "type_id",
            "type_name",
        ],
    );
    assert_eq!(location["name"], json!("Sell Hangar Container"));
    assert_eq!(
        location["station_name"],
        json!("Jita IV - Moon 4 - Caldari Navy Assembly Plant"),
        "the parent chain resolves the hosting station"
    );
    assert_eq!(location["abyssal_count"], json!(1));
    assert_eq!(location["public_asset_id"], serde_json::Value::Null);

    let (status, body) = send(&app, "GET", "/api/sell/modules", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_array().expect("entries").len(),
        0,
        "nothing published yet"
    );

    // Publishing the container makes the module sellable.
    let (status, _) = send(
        &app,
        "POST",
        "/public-assets",
        Some(&session),
        Some(json!({ "asset_id": container_asset_id })),
    )
    .await;
    assert!(status.is_redirection(), "publish redirects back: {status}");

    let (status, body) = send(&app, "GET", "/api/sell/locations", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body[0]["public_asset_id"].is_i64(),
        "the container is now published"
    );

    let (status, body) = send(&app, "GET", "/api/sell/modules", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    let entries = body.as_array().expect("entries");
    assert_eq!(entries.len(), 1);
    crate::common::assert_default_module_keys(&entries[0], true, &[]);
    assert_eq!(entries[0]["id"], json!(module.module_id));
    assert_eq!(
        entries[0]["asset"]["parent_name"],
        json!("Sell Hangar Container"),
        "the seller's own asset location rides on the module",
    );

    let (status, body) = send(&app, "GET", "/api/sell/page", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    let mut keys: Vec<&str> = body
        .as_object()
        .expect("page")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["character_id", "stats"]);
    let mut stats_keys: Vec<&str> = body["stats"]
        .as_object()
        .expect("stats")
        .keys()
        .map(String::as_str)
        .collect();
    stats_keys.sort_unstable();
    assert_eq!(
        stats_keys,
        [
            "average_value",
            "brownbars_count",
            "diamondbars_count",
            "goldbars_count",
            "total_count",
            "total_value",
        ]
    );
    assert_eq!(body["stats"]["total_count"], json!(1));
    assert_eq!(body["character_id"], json!(SELLER_CHARACTER));
}
