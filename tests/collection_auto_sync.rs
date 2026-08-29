//! Behavior tests for the collection auto-sync endpoints (legacy
//! `CollectionAutoSyncController` and SyncCollectionWithLocationsAction):
//! enabling/disabling auto-sync, managing tracked locations, and the
//! rebuild-from-locations sync semantics.
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
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

const WEBIFIER_TYPE_ID: i64 = 47702;
/// A fixture type reused as the ship/container hull type.
const HULL_TYPE_ID: i64 = 28514;

const OWNER_CHARACTER: i64 = 990_007_001;
/// The owner's second character; auto-sync scopes to the collection's
/// character only, unlike /collection-locations.
const OWNER_ALT_CHARACTER: i64 = 990_007_002;
const RIVAL_CHARACTER: i64 = 990_007_003;
const STATION_ID: i64 = 990_007_100;
const SHIP_ITEM: i64 = 990_007_201;
const CONTAINER_ITEM: i64 = 990_007_202;
const MODULE_AT_STATION: i64 = 990_007_301;
const MODULE_IN_SHIP: i64 = 990_007_302;
const MODULE_IN_CONTAINER: i64 = 990_007_303;
const ALT_MODULE_IN_SHIP: i64 = 990_007_304;

struct Seeded {
    owner_session: String,
    rival_session: String,
    collection_id: i64,
    collection_slug: String,
    ship_asset_id: i64,
    container_asset_id: i64,
}

static SEEDED: OnceCell<Seeded> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static Seeded {
    SEEDED.get_or_init(|| seed(pool)).await
}

async fn seed(pool: &PgPool) -> Seeded {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables)
        .await
        .expect("seed reference tables");

    for (table, column, base) in [
        ("assets", "item_id", 990_007_200i64),
        ("modules", "id", 990_007_300),
    ] {
        sqlx::query(&format!(
            "delete from {table} where {column} >= $1 and {column} < $1 + 200"
        ))
        .bind(base)
        .execute(pool)
        .await
        .expect("clean table");
    }
    sqlx::query("delete from collections where character_id = any($1)")
        .bind(vec![OWNER_CHARACTER, OWNER_ALT_CHARACTER, RIVAL_CHARACTER])
        .execute(pool)
        .await
        .expect("clean collections");
    sqlx::query("delete from stations where id = $1")
        .bind(STATION_ID)
        .execute(pool)
        .await
        .expect("clean station");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![OWNER_CHARACTER, OWNER_ALT_CHARACTER, RIVAL_CHARACTER])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Autosync Owner", "Autosync Rival"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Autosync Owner') returning id")
            .fetch_one(pool)
            .await
            .expect("create owner");
    let rival_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Autosync Rival') returning id")
            .fetch_one(pool)
            .await
            .expect("create rival");
    for (id, name, user_id) in [
        (OWNER_CHARACTER, "Autosync Main", owner_id),
        (OWNER_ALT_CHARACTER, "Autosync Alt", owner_id),
        (RIVAL_CHARACTER, "Autosync Rival Char", rival_id),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("create character");
    }

    sqlx::query(
        "insert into stations (id, name, type_id, solarsystem_id)
         values ($1, 'Autosync Station', null, 30000142)",
    )
    .bind(STATION_ID)
    .execute(pool)
    .await
    .expect("create station");

    type AssetSeed = (i64, i64, i64, i64, bool, &'static str);
    let assets: [AssetSeed; 6] = [
        (
            OWNER_CHARACTER,
            SHIP_ITEM,
            HULL_TYPE_ID,
            STATION_ID,
            false,
            "station",
        ),
        (
            OWNER_CHARACTER,
            CONTAINER_ITEM,
            HULL_TYPE_ID,
            SHIP_ITEM,
            false,
            "item",
        ),
        (
            OWNER_CHARACTER,
            MODULE_AT_STATION,
            WEBIFIER_TYPE_ID,
            STATION_ID,
            true,
            "station",
        ),
        (
            OWNER_CHARACTER,
            MODULE_IN_SHIP,
            WEBIFIER_TYPE_ID,
            SHIP_ITEM,
            true,
            "item",
        ),
        (
            OWNER_CHARACTER,
            MODULE_IN_CONTAINER,
            WEBIFIER_TYPE_ID,
            CONTAINER_ITEM,
            true,
            "item",
        ),
        (
            OWNER_ALT_CHARACTER,
            ALT_MODULE_IN_SHIP,
            WEBIFIER_TYPE_ID,
            SHIP_ITEM,
            true,
            "item",
        ),
    ];
    for (character_id, item_id, type_id, location_id, is_abyssal, location_type) in assets {
        sqlx::query(
            "insert into assets (character_id, item_id, type_id, location_id,
                                 is_abyssal, location_type, location_flag, quantity)
             values ($1, $2, $3, $4, $5, $6, 'Hangar', 1)",
        )
        .bind(character_id)
        .bind(item_id)
        .bind(type_id)
        .bind(location_id)
        .bind(is_abyssal)
        .bind(location_type)
        .execute(pool)
        .await
        .expect("create asset");
    }

    for module_id in [
        MODULE_AT_STATION,
        MODULE_IN_SHIP,
        MODULE_IN_CONTAINER,
        ALT_MODULE_IN_SHIP,
    ] {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(module_id)
            .bind(WEBIFIER_TYPE_ID)
            .execute(pool)
            .await
            .expect("create module");
    }

    let collection = mutamarket::collections::create_collection(
        pool,
        OWNER_CHARACTER,
        "Auto Synced",
        None,
        "private",
    )
    .await
    .expect("create collection");

    let asset_id = |item_id: i64| async move {
        sqlx::query_scalar::<_, i64>("select id from assets where item_id = $1")
            .bind(item_id)
            .fetch_one(pool)
            .await
            .expect("asset id")
    };

    Seeded {
        owner_session: create_session(pool, owner_id, Some(OWNER_CHARACTER))
            .await
            .expect("owner session"),
        rival_session: create_session(pool, rival_id, Some(RIVAL_CHARACTER))
            .await
            .expect("rival session"),
        collection_id: collection.id,
        collection_slug: collection.slug(),
        ship_asset_id: asset_id(SHIP_ITEM).await,
        container_asset_id: asset_id(CONTAINER_ITEM).await,
    }
}

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::REFERER, "/collections/x");
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let body = match body {
        Some(json) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).expect("valid request"))
        .await
        .expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        location,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn collection_state(pool: &PgPool, collection_id: i64) -> (bool, Option<String>) {
    sqlx::query_as::<_, (bool, Option<String>)>(
        "select auto_sync, last_synced_at::text from collections where id = $1",
    )
    .bind(collection_id)
    .fetch_one(pool)
    .await
    .expect("collection state")
}

async fn tracked_asset_ids(pool: &PgPool, collection_id: i64) -> Vec<i64> {
    sqlx::query_scalar(
        "select asset_id from collection_locations where collection_id = $1 order by asset_id",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .expect("tracked locations")
}

async fn collection_module_ids(pool: &PgPool, collection_id: i64) -> Vec<i64> {
    sqlx::query_scalar(
        "select module_id from collection_modules where collection_id = $1 order by module_id",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .expect("collection modules")
}

#[tokio::test]
async fn collection_auto_sync_lifecycle() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let seeded = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;
    let slug = &seeded.collection_slug;

    // Unknown collections 404, someone else's answer the policy 403.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/collections/nope/auto-sync",
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.rival_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Every invalid location_ids entry is reported, Laravel-style.
    let (status, _, body) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.owner_session),
        Some(json!({ "location_ids": [seeded.ship_asset_id, 0] })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body,
        json!({
            "message": "The given data was invalid.",
            "errors": { "location_ids.1": ["The selected location ids.1 is invalid."] },
        }),
    );

    // Enabling with no locations still runs the initial sync: the
    // legacy rebuild clears the current modules and stamps
    // last_synced_at even when nothing is tracked.
    mutamarket::collections::add_collection_module(
        &pool,
        seeded.collection_id,
        MODULE_AT_STATION,
        None,
    )
    .await
    .expect("preload module");
    let (status, location, _) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/collections/x"));
    let (auto_sync, last_synced_at) = collection_state(&pool, seeded.collection_id).await;
    assert!(auto_sync);
    assert!(last_synced_at.is_some());
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        Vec::<i64>::new()
    );

    // Disabling clears the tracked locations and the sync stamp.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let (auto_sync, last_synced_at) = collection_state(&pool, seeded.collection_id).await;
    assert!(!auto_sync);
    assert_eq!(last_synced_at, None);

    // Enabling with an initial location syncs its modules, scoped to the
    // collection's character only (the alt's module in the same ship
    // stays out, unlike /collection-locations).
    let (status, _, _) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.owner_session),
        Some(json!({ "location_ids": [seeded.ship_asset_id] })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        tracked_asset_ids(&pool, seeded.collection_id).await,
        vec![seeded.ship_asset_id],
    );
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_SHIP, MODULE_IN_CONTAINER],
    );

    // The tracked-location store validates its asset_id.
    let (status, _, body) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync/locations"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["errors"],
        json!({ "asset_id": ["The asset id field is required."] })
    );
    let (status, _, body) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync/locations"),
        Some(&seeded.owner_session),
        Some(json!({ "asset_id": 0 })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["errors"],
        json!({ "asset_id": ["The selected asset id is invalid."] })
    );

    // Tracking the container re-syncs; a module smuggled in manually is
    // swept away by the rebuild.
    mutamarket::collections::add_collection_module(
        &pool,
        seeded.collection_id,
        MODULE_AT_STATION,
        None,
    )
    .await
    .expect("smuggle module");
    let (status, _, _) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync/locations"),
        Some(&seeded.owner_session),
        Some(json!({ "asset_id": seeded.container_asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        tracked_asset_ids(&pool, seeded.collection_id).await,
        vec![seeded.ship_asset_id, seeded.container_asset_id],
    );
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_SHIP, MODULE_IN_CONTAINER],
    );

    // The tracked-location delete binds {asset} like the legacy implicit
    // binding: unknown (or non-numeric) ids 404 before the policy runs.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/collections/{slug}/auto-sync/locations/0"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/collections/{slug}/auto-sync/locations/abc"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!(
            "/collections/{slug}/auto-sync/locations/{}",
            seeded.ship_asset_id
        ),
        Some(&seeded.rival_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Untracking the ship re-syncs from the remaining container only.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!(
            "/collections/{slug}/auto-sync/locations/{}",
            seeded.ship_asset_id
        ),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        tracked_asset_ids(&pool, seeded.collection_id).await,
        vec![seeded.container_asset_id],
    );
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_CONTAINER],
    );

    // Disabling keeps the current modules (the legacy notification says
    // so explicitly).
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/collections/{slug}/auto-sync"),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        tracked_asset_ids(&pool, seeded.collection_id).await,
        Vec::<i64>::new()
    );
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_CONTAINER],
    );

    // Tracking a location while auto-sync is off records it but the sync
    // no-ops (the legacy isAutoSync guard): modules and the stamp stay.
    let (status, _, _) = send(
        &app,
        Method::POST,
        &format!("/collections/{slug}/auto-sync/locations"),
        Some(&seeded.owner_session),
        Some(json!({ "asset_id": seeded.ship_asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        tracked_asset_ids(&pool, seeded.collection_id).await,
        vec![seeded.ship_asset_id],
    );
    let (auto_sync, last_synced_at) = collection_state(&pool, seeded.collection_id).await;
    assert!(!auto_sync);
    assert_eq!(last_synced_at, None);
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_CONTAINER],
    );
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
async fn collection_page_carries_owner_location_data() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let seeded = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // A collection of its own so the lifecycle test cannot interfere.
    let collection = mutamarket::collections::create_collection(
        &pool,
        OWNER_CHARACTER,
        "Owner Page",
        None,
        "private",
    )
    .await
    .expect("create collection");
    mutamarket::collections::enable_auto_sync(
        &pool,
        collection.id,
        OWNER_CHARACTER,
        &[seeded.container_asset_id],
    )
    .await
    .expect("enable auto-sync");

    let (status, _, page) = send(
        &app,
        Method::GET,
        &format!("/api/collections/{}", collection.slug()),
        Some(&seeded.owner_session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["auto_sync"], json!(true));
    assert!(page["last_synced_at"].is_string());

    // The owner's location grid: the ship and the container hold the
    // collection character's abyssals (the alt's module in the same ship
    // does not count here, the legacy per-character scope).
    let locations = page["locations"].as_array().expect("locations");
    assert_eq!(locations.len(), 2);
    let by_item = |item_id: i64| {
        locations
            .iter()
            .find(|row| row["item_id"].as_i64() == Some(item_id))
            .unwrap_or_else(|| panic!("location row {item_id}"))
    };
    let ship = by_item(SHIP_ITEM);
    assert_eq!(
        sorted_keys(ship),
        [
            "asset_id",
            "corporation_id",
            "item_id",
            "location_id",
            "modules_count",
            "name",
            "public_asset_id",
            "slug",
            "station",
            "type_id",
            "type_name",
        ],
    );
    assert_eq!(ship["asset_id"].as_i64(), Some(seeded.ship_asset_id));
    assert_eq!(ship["modules_count"], json!(2));
    assert_eq!(ship["location_id"].as_i64(), Some(STATION_ID));
    assert_eq!(ship["station"]["name"], json!("Autosync Station"));
    assert_eq!(
        sorted_keys(&ship["station"]),
        ["id", "name", "slug", "type_id"]
    );
    let container = by_item(CONTAINER_ITEM);
    assert_eq!(container["modules_count"], json!(1));
    assert_eq!(container["location_id"].as_i64(), Some(SHIP_ITEM));
    assert_eq!(container["station"]["id"].as_i64(), Some(STATION_ID));

    // The tracked locations mirror collection_locations, counts unloaded
    // (0) like the legacy resource.
    let tracked = page["tracked_locations"].as_array().expect("tracked");
    assert_eq!(tracked.len(), 1);
    assert_eq!(
        tracked[0]["asset_id"].as_i64(),
        Some(seeded.container_asset_id)
    );
    assert_eq!(tracked[0]["item_id"].as_i64(), Some(CONTAINER_ITEM));
    assert_eq!(tracked[0]["modules_count"], json!(0));
}
