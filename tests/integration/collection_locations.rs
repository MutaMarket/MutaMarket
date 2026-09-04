//! Behavior tests for the /collection-locations endpoints (legacy
//! `CollectionLocationController`): bulk add, sync and remove of a
//! location's modules, with the legacy 404/403/422 order and quirks.
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

const OWNER_CHARACTER: i64 = 990_006_001;
/// The owner's second character; /collection-locations scopes to every
/// character of the user, unlike auto-sync.
const OWNER_ALT_CHARACTER: i64 = 990_006_002;
const RIVAL_CHARACTER: i64 = 990_006_003;
const STATION_ID: i64 = 990_006_100;
const SHIP_ITEM: i64 = 990_006_201;
const CONTAINER_ITEM: i64 = 990_006_202;
/// Abyssal modules: at the station, in the ship, in the container, the
/// alt's module in the ship, and the rival's module in the ship.
const MODULE_AT_STATION: i64 = 990_006_301;
const MODULE_IN_SHIP: i64 = 990_006_302;
const MODULE_IN_CONTAINER: i64 = 990_006_303;
const ALT_MODULE_IN_SHIP: i64 = 990_006_304;
const RIVAL_MODULE_IN_SHIP: i64 = 990_006_305;

struct Seeded {
    owner_session: String,
    rival_session: String,
    collection_id: i64,
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
        ("assets", "item_id", 990_006_200i64),
        ("modules", "id", 990_006_300),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "delete from {table} where {column} >= $1 and {column} < $1 + 200"
        )))
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
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![OWNER_CHARACTER, OWNER_ALT_CHARACTER, RIVAL_CHARACTER])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Colloc Owner", "Colloc Rival"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Colloc Owner') returning id")
            .fetch_one(pool)
            .await
            .expect("create owner");
    let rival_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Colloc Rival') returning id")
            .fetch_one(pool)
            .await
            .expect("create rival");
    for (id, name, user_id) in [
        (OWNER_CHARACTER, "Colloc Main", owner_id),
        (OWNER_ALT_CHARACTER, "Colloc Alt", owner_id),
        (RIVAL_CHARACTER, "Colloc Rival Char", rival_id),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("create character");
    }

    // The asset tree: the owner's ship at the station holds a container,
    // a module, the alt's module and the rival's module; one owner module
    // sits loose at the station; one module sits in the container.
    type AssetSeed = (i64, i64, i64, i64, bool, &'static str);
    let assets: [AssetSeed; 7] = [
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
        (
            RIVAL_CHARACTER,
            RIVAL_MODULE_IN_SHIP,
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
        RIVAL_MODULE_IN_SHIP,
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
        "Location Bulk",
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
async fn collection_location_bulk_actions() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let seeded = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // A missing or unknown collection_id 404s before validation, the
    // legacy authorize-findOrFail order.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": seeded.ship_asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": seeded.ship_asset_id, "collection_id": 0 })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Someone else's collection answers the policy 403.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.rival_session),
        Some(json!({ "location_id": seeded.ship_asset_id, "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The location_id rules answer the Laravel 422 shape.
    let (status, _, body) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body,
        json!({
            "message": "The given data was invalid.",
            "errors": { "location_id": ["The location id field is required."] },
        }),
    );
    let (status, _, body) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": 0, "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["errors"],
        json!({ "location_id": ["The selected location id is invalid."] }),
    );

    // Adding the ship pulls in everything nested below it across every
    // character of the user (the alt's module included), but never the
    // rival's; the response is the legacy redirect back.
    let (status, location, _) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": seeded.ship_asset_id, "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/collections/x"));
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_SHIP, MODULE_IN_CONTAINER, ALT_MODULE_IN_SHIP],
    );

    // Re-adding is a no-op (legacy insertOrIgnore).
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": seeded.ship_asset_id, "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_SHIP, MODULE_IN_CONTAINER, ALT_MODULE_IN_SHIP],
    );

    // Removing the container only drops what sits inside it.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({
            "location_id": seeded.container_asset_id,
            "collection_id": seeded.collection_id,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_SHIP, ALT_MODULE_IN_SHIP],
    );

    // Legacy quirk: the remove filters only the location ancestor to the
    // user, not the module's own asset row, so a rival-owned module
    // inside the user's ship is removed with the rest.
    mutamarket::collections::add_collection_module(
        &pool,
        seeded.collection_id,
        RIVAL_MODULE_IN_SHIP,
        None,
    )
    .await
    .expect("add rival module");
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({ "location_id": seeded.ship_asset_id, "collection_id": seeded.collection_id })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        Vec::<i64>::new()
    );

    // The PUT sync replaces whatever the collection held with the
    // location's modules.
    mutamarket::collections::add_collection_module(
        &pool,
        seeded.collection_id,
        MODULE_AT_STATION,
        None,
    )
    .await
    .expect("add station module");
    let (status, _, _) = send(
        &app,
        Method::PUT,
        "/collection-locations",
        Some(&seeded.owner_session),
        Some(json!({
            "location_id": seeded.container_asset_id,
            "collection_id": seeded.collection_id,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        collection_module_ids(&pool, seeded.collection_id).await,
        vec![MODULE_IN_CONTAINER],
    );
}
