//! Behavior tests for the asset-location pages (legacy
//! `LocationController` / `LocationCollectionController`): the tree
//! payload, the recursive per-location module scope, the stats, and
//! the collection-from-location action.
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

const WEBIFIER_TYPE_ID: i64 = 47702;
/// A fixture type reused as the ship/container hull type.
const HULL_TYPE_ID: i64 = 28514;

const OWNER_CHARACTER: i64 = 990_005_001;
const RIVAL_CHARACTER: i64 = 990_005_002;
const STATION_ID: i64 = 990_005_100;
const SHIP_ITEM: i64 = 990_005_201;
const CONTAINER_ITEM: i64 = 990_005_202;
const EMPTY_CONTAINER_ITEM: i64 = 990_005_203;
/// Abyssal modules: at the station, in the ship, in the container.
const MODULE_AT_STATION: i64 = 990_005_301;
const MODULE_IN_SHIP: i64 = 990_005_302;
const MODULE_IN_CONTAINER: i64 = 990_005_303;
const RIVAL_MODULE: i64 = 990_005_304;
/// A structure id no table knows about, holding one module.
const UNRESOLVED_STRUCTURE: i64 = 990_005_101;
const MODULE_IN_STRUCTURE: i64 = 990_005_305;

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

    for (table, column, base) in [
        ("assets", "item_id", 990_005_200i64),
        ("modules", "id", 990_005_300),
        ("collections", "character_id", OWNER_CHARACTER),
    ] {
        sqlx::query(&format!(
            "delete from {table} where {column} >= $1 and {column} < $1 + 200"
        ))
        .bind(base)
        .execute(pool)
        .await
        .expect("clean table");
    }
    sqlx::query("delete from stations where id = $1")
        .bind(STATION_ID)
        .execute(pool)
        .await
        .expect("clean station");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![OWNER_CHARACTER, RIVAL_CHARACTER])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Location Owner", "Location Rival"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Location Owner') returning id")
            .fetch_one(pool)
            .await
            .expect("create owner");
    let rival_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Location Rival') returning id")
            .fetch_one(pool)
            .await
            .expect("create rival");
    for (id, name, user_id) in [
        (OWNER_CHARACTER, "Locfix Owner", owner_id),
        (RIVAL_CHARACTER, "Locfix Rival", rival_id),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("create character");
    }

    sqlx::query("insert into stations (id, name, type_id, solarsystem_id) values ($1, 'Jita IV - Moon 4', null, 30000142)")
        .bind(STATION_ID)
        .execute(pool)
        .await
        .expect("create station");

    // The asset tree: ship at the station holding a named container and
    // a module; one module loose at the station; an empty container that
    // must not appear in the tree; a rival module that must never leak.
    type AssetSeed = (i64, i64, i64, i64, Option<&'static str>, bool, &'static str);
    let assets: [AssetSeed; 6] = [
        (
            OWNER_CHARACTER,
            SHIP_ITEM,
            HULL_TYPE_ID,
            STATION_ID,
            Some("My Hauler"),
            false,
            "station",
        ),
        (
            OWNER_CHARACTER,
            CONTAINER_ITEM,
            HULL_TYPE_ID,
            SHIP_ITEM,
            None,
            false,
            "item",
        ),
        (
            OWNER_CHARACTER,
            EMPTY_CONTAINER_ITEM,
            HULL_TYPE_ID,
            STATION_ID,
            None,
            false,
            "station",
        ),
        (
            OWNER_CHARACTER,
            MODULE_AT_STATION,
            WEBIFIER_TYPE_ID,
            STATION_ID,
            None,
            true,
            "station",
        ),
        (
            OWNER_CHARACTER,
            MODULE_IN_SHIP,
            WEBIFIER_TYPE_ID,
            SHIP_ITEM,
            None,
            true,
            "item",
        ),
        (
            OWNER_CHARACTER,
            MODULE_IN_CONTAINER,
            WEBIFIER_TYPE_ID,
            CONTAINER_ITEM,
            None,
            true,
            "item",
        ),
    ];
    for (character_id, item_id, type_id, location_id, name, is_abyssal, location_type) in assets {
        sqlx::query(
            "insert into assets (character_id, item_id, type_id, location_id, name,
                                 is_abyssal, location_type, location_flag, quantity)
             values ($1, $2, $3, $4, $5, $6, $7, 'Hangar', 1)",
        )
        .bind(character_id)
        .bind(item_id)
        .bind(type_id)
        .bind(location_id)
        .bind(name)
        .bind(is_abyssal)
        .bind(location_type)
        .execute(pool)
        .await
        .expect("create asset");
    }
    sqlx::query(
        "insert into assets (character_id, item_id, type_id, location_id,
                             is_abyssal, location_type, location_flag, quantity)
         values ($1, $2, $3, $4, true, 'item', 'Hangar', 1)",
    )
    .bind(OWNER_CHARACTER)
    .bind(MODULE_IN_STRUCTURE)
    .bind(WEBIFIER_TYPE_ID)
    .bind(UNRESOLVED_STRUCTURE)
    .execute(pool)
    .await
    .expect("create structure asset");
    sqlx::query(
        "insert into assets (character_id, item_id, type_id, location_id,
                             is_abyssal, location_type, location_flag, quantity)
         values ($1, $2, $3, $4, true, 'station', 'Hangar', 1)",
    )
    .bind(RIVAL_CHARACTER)
    .bind(RIVAL_MODULE)
    .bind(WEBIFIER_TYPE_ID)
    .bind(STATION_ID)
    .execute(pool)
    .await
    .expect("create rival asset");

    for (module_id, value) in [
        (MODULE_AT_STATION, Some(100_000_000.0)),
        (MODULE_IN_SHIP, Some(50_000_000.0)),
        (MODULE_IN_CONTAINER, None),
        (RIVAL_MODULE, Some(999_000_000.0)),
        (MODULE_IN_STRUCTURE, None),
    ] {
        sqlx::query("insert into modules (id, type_id, estimated_value) values ($1, $2, $3)")
            .bind(module_id)
            .bind(WEBIFIER_TYPE_ID)
            .bind(value)
            .execute(pool)
            .await
            .expect("create module");
    }

    create_session(pool, owner_id, Some(OWNER_CHARACTER))
        .await
        .expect("session")
}

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(uri);
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

fn ids(value: &serde_json::Value) -> Vec<i64> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect()
}

#[tokio::test]
async fn locations_tree_and_membership() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let session = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests get the JSON 401.
    let (status, _, body) = request(&app, Method::GET, "/api/locations", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));

    // The tree payload: the ship and its container hold abyssals, the
    // empty container does not appear; the station roots the tree.
    let (status, _, body) = request(&app, Method::GET, "/api/locations", Some(session), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec![
            "location_modules_count",
            "locations",
            "stations",
            "structures"
        ],
    );
    let locations = body["locations"].as_array().expect("locations");
    let location_ids: Vec<i64> = locations
        .iter()
        .filter_map(|location| location["id"].as_i64())
        .collect();
    assert!(location_ids.contains(&SHIP_ITEM));
    assert!(location_ids.contains(&CONTAINER_ITEM));
    assert!(
        !location_ids.contains(&EMPTY_CONTAINER_ITEM),
        "empty containers stay hidden"
    );
    let ship = locations
        .iter()
        .find(|location| location["id"].as_i64() == Some(SHIP_ITEM))
        .expect("ship row");
    assert_eq!(
        sorted_keys(ship),
        vec![
            "character_id",
            "corporation_id",
            "id",
            "location",
            "name",
            "slug",
            "type"
        ],
    );
    assert_eq!(
        ship["slug"].as_str(),
        Some(format!("my-hauler-{SHIP_ITEM}").as_str())
    );
    assert_eq!(ship["location"]["id"].as_i64(), Some(STATION_ID));

    let stations = body["stations"].as_array().expect("stations");
    assert!(
        stations
            .iter()
            .any(|station| station["id"].as_i64() == Some(STATION_ID)),
        "the hosting station roots the tree",
    );
    let counts = body["location_modules_count"].as_object().expect("counts");
    assert_eq!(counts[&STATION_ID.to_string()].as_i64(), Some(1));
    assert_eq!(counts[&SHIP_ITEM.to_string()].as_i64(), Some(1));
    assert_eq!(counts[&CONTAINER_ITEM.to_string()].as_i64(), Some(1));

    // An unresolved structure (in no table) still roots the tree as a
    // placeholder and stays browsable.
    let structures = body["structures"].as_array().expect("structures");
    let placeholder = structures
        .iter()
        .find(|structure| structure["id"].as_i64() == Some(UNRESOLVED_STRUCTURE))
        .expect("placeholder root");
    assert!(placeholder["name"].is_null());
    assert_eq!(
        placeholder["slug"].as_str(),
        Some(format!("unknown-structure-{UNRESOLVED_STRUCTURE}").as_str()),
    );
    let (status, _, shown) = request(
        &app,
        Method::GET,
        &format!("/api/locations/unknown-structure-{UNRESOLVED_STRUCTURE}"),
        Some(session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ids(&shown["modules"]), vec![MODULE_IN_STRUCTURE]);
    assert_eq!(
        shown["location"]["type"]["name"].as_str(),
        Some("Structure")
    );

    // The station shows everything nested below it, newest module id
    // first, with the location stats; the rival's module never leaks.
    let (status, _, body) = request(
        &app,
        Method::GET,
        &format!("/api/locations/jita-iv-moon-4-{STATION_ID}"),
        Some(session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec!["available_types", "location", "modules", "stats"],
    );
    assert_eq!(
        ids(&body["modules"]),
        vec![MODULE_IN_CONTAINER, MODULE_IN_SHIP, MODULE_AT_STATION],
    );
    // withDefaultRelations: every card carries the owner's asset row, so
    // the location page shows where each module sits.
    for module in body["modules"].as_array().expect("modules") {
        crate::common::assert_default_module_keys(module, true, &[]);
        assert!(module["asset"].is_object(), "owned module carries its asset");
    }
    let asset_of = |id: i64| {
        body["modules"]
            .as_array()
            .expect("modules")
            .iter()
            .find(|module| module["id"].as_i64() == Some(id))
            .map(|module| module["asset"].clone())
            .expect("module listed")
    };
    assert_eq!(
        asset_of(MODULE_AT_STATION)["parent_name"],
        serde_json::json!("Jita IV - Moon 4")
    );
    assert_eq!(
        asset_of(MODULE_IN_SHIP)["parent_name"],
        serde_json::json!("My Hauler")
    );
    assert_eq!(body["stats"]["total_count"].as_i64(), Some(3));
    assert_eq!(body["stats"]["total_value"].as_f64(), Some(150_000_000.0));
    assert_eq!(body["location"]["name"].as_str(), Some("Jita IV - Moon 4"));
    assert_eq!(body["available_types"].as_array().expect("types").len(), 1);

    // The ship scopes to its own contents and carries the breadcrumb.
    let (_, _, body) = request(
        &app,
        Method::GET,
        &format!("/api/locations/my-hauler-{SHIP_ITEM}"),
        Some(session),
        None,
    )
    .await;
    assert_eq!(
        ids(&body["modules"]),
        vec![MODULE_IN_CONTAINER, MODULE_IN_SHIP]
    );
    assert_eq!(
        body["location"]["location"]["id"].as_i64(),
        Some(STATION_ID)
    );
    assert_eq!(
        body["location"]["location"]["type"]["name"].as_str(),
        Some("Jita IV - Moon 4"),
    );

    // The query segment filters like every browser page.
    let (_, _, body) = request(
        &app,
        Method::GET,
        &format!("/api/locations/x-{SHIP_ITEM}/type/{WEBIFIER_TYPE_ID}/sort/value/desc"),
        Some(session),
        None,
    )
    .await;
    assert_eq!(
        ids(&body["modules"]),
        vec![MODULE_IN_SHIP, MODULE_IN_CONTAINER]
    );

    // Unknown ids are the legacy 404.
    let (status, _, body) = request(
        &app,
        Method::GET,
        "/api/locations/nowhere-1",
        Some(session),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"].as_str(),
        Some("This location does not exist.")
    );
}

#[tokio::test]
async fn location_collections_capture_the_contents() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let session = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests bounce to the login page.
    let (status, location, _) = request(
        &app,
        Method::POST,
        "/location-collections",
        None,
        Some(serde_json::json!({ "location_id": SHIP_ITEM })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));

    // The ship becomes a private collection of its modules, and the
    // response is the legacy redirect to it.
    let (status, location, _) = request(
        &app,
        Method::POST,
        "/location-collections",
        Some(session),
        Some(serde_json::json!({ "location_id": SHIP_ITEM })),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = location.expect("redirect target");
    assert!(
        location.starts_with("/collections/my-hauler-"),
        "got {location}"
    );

    let (name, visibility, count): (String, String, i64) = sqlx::query_as(
        "select c.name, c.visibility, count(cm.id)
         from collections c
         left join collection_modules cm on cm.collection_id = c.id
         where c.character_id = $1
         group by c.id
         order by c.id desc limit 1",
    )
    .bind(OWNER_CHARACTER)
    .fetch_one(&pool)
    .await
    .expect("collection row");
    assert_eq!(name, "My Hauler");
    assert_eq!(visibility, "private");
    assert_eq!(count, 2, "the ship's two nested modules");
}
