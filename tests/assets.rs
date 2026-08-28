//! Behavior tests for character asset ingestion against a mock ESI: the
//! kept subset (abyssal modules plus their container chain), player names,
//! module ingestion through the shared import path, structure resolution
//! for asset locations, the diff-delete on refresh, the import state
//! machine, and the stale-import sweeper.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::Path as AxumPath;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mutamarket::assets::{
    fail_stale_asset_imports, pending_asset_characters, status, step, sync_character_assets,
};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;

const OWNER_CHARACTER: i64 = 94_000_001;
const ACCESS_TOKEN: &str = "assets-access";
/// The assembled ship holding one of the modules.
const SHIP_ITEM: i64 = 5_001;
const SHIP_TYPE: i64 = 670;
/// Jita IV-4, the NPC station the ship is docked in.
const STATION: i64 = 60_003_760;
/// The player structure the second module sits in directly.
const STRUCTURE: i64 = 1_040_000_001;

fn asset(item_id: i64, type_id: i64, location_id: i64, location_type: &str, flag: &str, singleton: bool) -> serde_json::Value {
    json!({
        "item_id": item_id,
        "type_id": type_id,
        "location_id": location_id,
        "location_type": location_type,
        "location_flag": flag,
        "quantity": 1,
        "is_singleton": singleton,
    })
}

/// Mock ESI: character assets (second pass loses the structure module),
/// asset names, structure detail and the dynamic items of the fixture
/// modules. Every authenticated route checks the bearer token.
fn mock_esi(
    second_pass: Arc<AtomicBool>,
    ship_module: serde_json::Value,
    structure_module: serde_json::Value,
) -> Router {
    let ship_module_item = ship_module["item_id"].as_i64().expect("item id");
    let ship_module_type = ship_module["type_id"].as_i64().expect("type id");
    let structure_module_item = structure_module["item_id"].as_i64().expect("item id");
    let structure_module_type = structure_module["type_id"].as_i64().expect("type id");

    let bearer_ok = |headers: &HeaderMap| {
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some(&format!("Bearer {ACCESS_TOKEN}"))
    };

    Router::new()
        .route(
            "/latest/characters/{character_id}/assets/",
            get(move |headers: HeaderMap| {
                let second_pass = second_pass.clone();
                async move {
                    if !bearer_ok(&headers) {
                        return StatusCode::FORBIDDEN.into_response();
                    }

                    // The ship sits inside an office-like wrapper ESI
                    // cannot name; the names fetch must bisect around it.
                    let wrapper = asset(UNNAMEABLE_ITEM, 27, STATION, "station", "OfficeFolder", true);
                    let ship = asset(SHIP_ITEM, SHIP_TYPE, UNNAMEABLE_ITEM, "item", "Hangar", true);
                    let fitted = asset(ship_module_item, ship_module_type, SHIP_ITEM, "item", "MedSlot1", true);
                    let loose = asset(structure_module_item, structure_module_type, STRUCTURE, "item", "Hangar", true);
                    // A non-abyssal stack that must not be kept.
                    let minerals = json!({
                        "item_id": 9_001, "type_id": 34, "location_id": STATION,
                        "location_type": "station", "location_flag": "Hangar",
                        "quantity": 1_000_000, "is_singleton": false,
                    });

                    let feed = if second_pass.load(Ordering::SeqCst) {
                        json!([ship, fitted, minerals])
                    } else {
                        json!([ship, fitted, wrapper, loose, minerals])
                    };

                    ([("x-pages", "1")], Json(feed)).into_response()
                }
            }),
        )
        .route(
            "/latest/characters/{character_id}/assets/names/",
            post(move |headers: HeaderMap, Json(ids): Json<Vec<i64>>| async move {
                if !bearer_ok(&headers) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                if ids.contains(&UNNAMEABLE_ITEM) {
                    return StatusCode::NOT_FOUND.into_response();
                }
                let names: Vec<serde_json::Value> = ids
                    .iter()
                    .map(|id| {
                        json!({
                            "item_id": id,
                            "name": if *id == SHIP_ITEM { "Roll Boat" } else { "None" },
                        })
                    })
                    .collect();
                Json(names).into_response()
            }),
        )
        .route(
            "/latest/universe/structures/{structure_id}/",
            get(move |headers: HeaderMap| async move {
                if !bearer_ok(&headers) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                Json(json!({
                    "name": "Asset Home Astrahus",
                    "owner_id": 98_000_002,
                    "solar_system_id": 30_000_142,
                    "type_id": 35_832,
                }))
                .into_response()
            }),
        )
        .route(
            "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
            get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
                let ship_module = ship_module.clone();
                let structure_module = structure_module.clone();
                async move {
                    for module in [&ship_module, &structure_module] {
                        if module["type_id"] == json!(type_id) && module["item_id"] == json!(item_id)
                        {
                            return Json(module["dogma"].clone()).into_response();
                        }
                    }
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
}

fn dogma_payload(fixture_type_id: i64, module: &common::ModuleFixture) -> serde_json::Value {
    json!({
        "type_id": fixture_type_id,
        "item_id": module.module_id,
        "dogma": {
            "created_by": module.creator_id,
            "mutator_type_id": module.mutaplasmid_id,
            "source_type_id": module.source_type_id,
            "dogma_attributes": module
                .input_attributes
                .iter()
                .map(|attribute| json!({
                    "attribute_id": attribute.attribute_id,
                    "value": attribute.value,
                }))
                .collect::<Vec<_>>(),
        },
    })
}

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

async fn seed_character(pool: &PgPool, character_id: i64, scopes: &[&str], access_token: &str) {
    sqlx::query("insert into characters (id, name) values ($1, 'Asset Pilot') on conflict (id) do nothing")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("seed character");

    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("clean tokens");

    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, $2, 'refresh', 'Bearer', 'owner', $3, now() + interval '20 minutes')",
    )
    .bind(character_id)
    .bind(access_token)
    .bind(scopes.iter().map(|scope| scope.to_string()).collect::<Vec<_>>())
    .execute(pool)
    .await
    .expect("seed token");
}

async fn setup(character_id: i64) -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");

    // The nameable-type filter: the ship hull sits under the Ships market
    // group; the office wrapper (type 27) stays outside it.
    sqlx::query(
        "insert into market_groups (id, parent_id) values (4, null), (1361, 4)
         on conflict (id) do update set parent_id = excluded.parent_id",
    )
    .execute(&pool)
    .await
    .expect("seed market groups");
    sqlx::query(
        "insert into types (id, name, published, market_group_id)
         values ($1, 'Capsule', true, 1361)
         on conflict (id) do update set market_group_id = 1361, published = true",
    )
    .bind(SHIP_TYPE)
    .execute(&pool)
    .await
    .expect("mark ship nameable");
    let reference = ReferenceData::from_tables(tables);

    // Idempotent across runs: this character's assets and imports reset.
    sqlx::query("update characters set latest_asset_import_id = null where id = $1")
        .bind(character_id)
        .execute(&pool)
        .await
        .expect("unlink import");
    sqlx::query("delete from assets where character_id = $1")
        .bind(character_id)
        .execute(&pool)
        .await
        .expect("clean assets");
    sqlx::query("delete from asset_imports where character_id = $1")
        .bind(character_id)
        .execute(&pool)
        .await
        .expect("clean imports");

    (pool, reference)
}

fn sso_stub(base: &str) -> SsoClient {
    SsoClient::new(base, "client", "secret", "http://test/eve/callback")
}

/// Estimator stub: untrained types never call it; anything else gets a
/// fast connection refusal and the estimate is skipped.
/// A singleton wrapper item ESI refuses to name (like a corp Office);
/// batches containing it must be bisected, not failed.
const UNNAMEABLE_ITEM: i64 = 5_900;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

#[tokio::test]
async fn asset_imports_keep_the_module_chain_and_recover_from_moves() {
    let (pool, reference) = setup(OWNER_CHARACTER).await;
    // Only this test touches the structure; cleaning it here keeps the
    // parallel tests from racing the delete.
    sqlx::query("delete from structures where id = $1")
        .bind(STRUCTURE)
        .execute(&pool)
        .await
        .expect("clean structure");
    seed_character(
        &pool,
        OWNER_CHARACTER,
        &["esi-assets.read_assets.v1", "esi-structures.read_character.v1"],
        ACCESS_TOKEN,
    )
    .await;

    let fixtures = common::load_module_fixtures();
    let ship_fixture = fixtures.iter().find(|f| f.type_id == 47736).expect("fixture");
    let structure_fixture = fixtures.iter().find(|f| f.type_id == 47740).expect("fixture");
    let ship_module = &ship_fixture.modules[1];
    let structure_module = &structure_fixture.modules[1];

    let second_pass = Arc::new(AtomicBool::new(false));
    let esi_url = start_mock(mock_esi(
        second_pass.clone(),
        dogma_payload(ship_fixture.type_id, ship_module),
        dogma_payload(structure_fixture.type_id, structure_module),
    ))
    .await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    // A character with the scope and no import yet is pending.
    assert!(
        pending_asset_characters(&pool)
            .await
            .expect("pending characters")
            .contains(&OWNER_CHARACTER),
    );

    let stats = sync_character_assets(&pool, &reference, &esi, &sso, &estimator_stub(), OWNER_CHARACTER)
        .await
        .expect("asset sync");
    assert_eq!(
        (stats.assets, stats.corporation_assets, stats.abyssal_modules, stats.modules_imported, stats.modules_failed),
        (4, 0, 2, 2, 0),
    );

    // Exactly the module chain is stored, containers before contents; the
    // mineral stack and the station are not rows.
    type AssetRow = (i64, i64, Option<String>, i64, String, String, bool, i64, Option<i64>);
    let rows: Vec<AssetRow> =
        sqlx::query_as(
            "select item_id, type_id, name, location_id, location_flag, location_type,
                    is_abyssal, index, corporation_id
             from assets where character_id = $1 order by item_id",
        )
        .bind(OWNER_CHARACTER)
        .fetch_all(&pool)
        .await
        .expect("asset rows");
    assert_eq!(
        rows,
        vec![
            (
                SHIP_ITEM,
                SHIP_TYPE,
                Some("Roll Boat".to_owned()),
                UNNAMEABLE_ITEM,
                "Hangar".to_owned(),
                "item".to_owned(),
                false,
                0,
                None,
            ),
            (
                UNNAMEABLE_ITEM,
                27,
                None,
                STATION,
                "OfficeFolder".to_owned(),
                "station".to_owned(),
                false,
                0,
                None,
            ),
            (
                ship_module.module_id,
                ship_fixture.type_id,
                None,
                SHIP_ITEM,
                "MedSlot1".to_owned(),
                "item".to_owned(),
                true,
                0,
                None,
            ),
            (
                structure_module.module_id,
                structure_fixture.type_id,
                None,
                STRUCTURE,
                "Hangar".to_owned(),
                "item".to_owned(),
                true,
                0,
                None,
            ),
        ],
    );

    // The abyssal modules went through the shared ingestion path.
    for module in [ship_module, structure_module] {
        let exists: Option<i64> = sqlx::query_scalar("select id from modules where id = $1")
            .bind(module.module_id)
            .fetch_optional(&pool)
            .await
            .expect("module row");
        assert_eq!(exists, Some(module.module_id));
    }

    // The import completed with its counts and is linked on the character.
    let (import_status, import_step, assets_count, corp_count, modules_count, imported, failed): (
        String,
        String,
        i32,
        i32,
        i32,
        i32,
        i32,
    ) = sqlx::query_as(
        "select ai.status, ai.step, ai.assets_count, ai.assets_corporation_count,
                ai.abyssal_modules_count, ai.abyssal_modules_imported_count,
                ai.abyssal_modules_failed_count
         from asset_imports ai
         join characters c on c.latest_asset_import_id = ai.id
         where c.id = $1",
    )
    .bind(OWNER_CHARACTER)
    .fetch_one(&pool)
    .await
    .expect("import row");
    assert_eq!(import_status, status::COMPLETED);
    assert_eq!(import_step, step::IMPORTING_ABYSSAL_MODULES);
    assert_eq!((assets_count, corp_count, modules_count, imported, failed), (4, 0, 2, 2, 0));

    // The structure hosting the loose module got resolved on the way.
    let structure_name: Option<String> =
        sqlx::query_scalar("select name from structures where id = $1")
            .bind(STRUCTURE)
            .fetch_one(&pool)
            .await
            .expect("structure row");
    assert_eq!(structure_name.as_deref(), Some("Asset Home Astrahus"));

    // A fresh import exists now, so the character is no longer pending.
    assert!(
        !pending_asset_characters(&pool)
            .await
            .expect("pending characters")
            .contains(&OWNER_CHARACTER),
    );

    // Auto-sync collections ride the import: one tracks the ship and the
    // structure module's own asset row, one is manual and must be left
    // alone (only auto_sync collections are picked up, and only after an
    // import that found modules — the legacy SyncAutoSyncCollectionsJob
    // dispatched from the module batch's finally()).
    sqlx::query("delete from collections where character_id = $1")
        .bind(OWNER_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean collections");
    let ship_asset_id: i64 =
        sqlx::query_scalar("select id from assets where character_id = $1 and item_id = $2")
            .bind(OWNER_CHARACTER)
            .bind(SHIP_ITEM)
            .fetch_one(&pool)
            .await
            .expect("ship asset id");
    let structure_module_asset_id: i64 =
        sqlx::query_scalar("select id from assets where character_id = $1 and item_id = $2")
            .bind(OWNER_CHARACTER)
            .bind(structure_module.module_id)
            .fetch_one(&pool)
            .await
            .expect("structure module asset id");
    let auto_collection = mutamarket::collections::create_collection(
        &pool,
        OWNER_CHARACTER,
        "Auto Hangar",
        None,
        "private",
    )
    .await
    .expect("create auto collection");
    mutamarket::collections::enable_auto_sync(
        &pool,
        auto_collection.id,
        OWNER_CHARACTER,
        &[ship_asset_id, structure_module_asset_id],
    )
    .await
    .expect("enable auto-sync");
    let manual_collection = mutamarket::collections::create_collection(
        &pool,
        OWNER_CHARACTER,
        "Manual Keepsakes",
        None,
        "private",
    )
    .await
    .expect("create manual collection");
    mutamarket::collections::add_collection_module(
        &pool,
        manual_collection.id,
        structure_module.module_id,
        None,
    )
    .await
    .expect("fill manual collection");

    let collection_modules = |collection_id: i64| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "select module_id from collection_modules
                 where collection_id = $1 order by module_id",
            )
            .bind(collection_id)
            .fetch_all(&pool)
            .await
            .expect("collection modules")
        }
    };
    let mut expected = vec![ship_module.module_id, structure_module.module_id];
    expected.sort_unstable();
    assert_eq!(collection_modules(auto_collection.id).await, expected);
    let synced_before: Option<String> = sqlx::query_scalar(
        "select last_synced_at::text from collections where id = $1",
    )
    .bind(auto_collection.id)
    .fetch_one(&pool)
    .await
    .expect("initial sync stamp");
    assert!(synced_before.is_some());

    // Second pass: the structure module left the hangar; its row (and the
    // now moduleless structure chain entry) disappears, the rest stays.
    second_pass.store(true, Ordering::SeqCst);
    let stats = sync_character_assets(&pool, &reference, &esi, &sso, &estimator_stub(), OWNER_CHARACTER)
        .await
        .expect("second sync");
    assert_eq!((stats.assets, stats.abyssal_modules), (2, 1));

    let remaining: Vec<i64> = sqlx::query_scalar(
        "select item_id from assets where character_id = $1 order by item_id",
    )
    .bind(OWNER_CHARACTER)
    .fetch_all(&pool)
    .await
    .expect("remaining assets");
    assert_eq!(remaining, vec![SHIP_ITEM, ship_module.module_id]);

    // The import re-synced the auto-sync collection: the vanished asset's
    // tracked location cascaded away and the rebuild kept only the ship's
    // module; the manual collection was not touched (its module row
    // survives asset deletion).
    assert_eq!(collection_modules(auto_collection.id).await, vec![ship_module.module_id]);
    let tracked: Vec<i64> = sqlx::query_scalar(
        "select asset_id from collection_locations where collection_id = $1",
    )
    .bind(auto_collection.id)
    .fetch_all(&pool)
    .await
    .expect("tracked locations");
    assert_eq!(tracked, vec![ship_asset_id]);
    let synced_after: Option<String> = sqlx::query_scalar(
        "select last_synced_at::text from collections where id = $1",
    )
    .bind(auto_collection.id)
    .fetch_one(&pool)
    .await
    .expect("second sync stamp");
    assert!(synced_after.is_some());
    assert_ne!(synced_before, synced_after, "the import stamped a fresh sync");
    assert_eq!(
        collection_modules(manual_collection.id).await,
        vec![structure_module.module_id],
    );
}

#[tokio::test]
async fn denied_asset_fetches_fail_the_import_and_drop_the_token() {
    const DENIED_CHARACTER: i64 = 94_000_002;
    let (pool, reference) = setup(DENIED_CHARACTER).await;
    // The stored token does not match what the mock expects: ESI answers
    // 403 like it does for a deauthorized application.
    seed_character(&pool, DENIED_CHARACTER, &["esi-assets.read_assets.v1"], "revoked-access").await;

    let esi_url = start_mock(mock_esi(
        Arc::new(AtomicBool::new(false)),
        json!({"item_id": 1, "type_id": 1}),
        json!({"item_id": 2, "type_id": 2}),
    ))
    .await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    sync_character_assets(&pool, &reference, &esi, &sso, &estimator_stub(), DENIED_CHARACTER)
        .await
        .expect_err("denied fetch must fail the import");

    let import_status: String = sqlx::query_scalar(
        "select status from asset_imports where character_id = $1 order by id desc limit 1",
    )
    .bind(DENIED_CHARACTER)
    .fetch_one(&pool)
    .await
    .expect("import row");
    assert_eq!(import_status, status::FAILED);

    // The token went the way of the legacy connector's 403 handling.
    let tokens: i64 = sqlx::query_scalar("select count(*) from esi_tokens where character_id = $1")
        .bind(DENIED_CHARACTER)
        .fetch_one(&pool)
        .await
        .expect("token count");
    assert_eq!(tokens, 0);
}

#[tokio::test]
async fn stale_imports_are_swept_to_failed() {
    const STALE_CHARACTER: i64 = 94_000_003;
    let (pool, _reference) = setup(STALE_CHARACTER).await;

    sqlx::query("insert into characters (id, name) values ($1, 'Stale Pilot') on conflict (id) do nothing")
        .bind(STALE_CHARACTER)
        .execute(&pool)
        .await
        .expect("seed character");

    let stale: i64 = sqlx::query_scalar(
        "insert into asset_imports (character_id, status, step, updated_at)
         values ($1, $2, $3, now() - interval '31 minutes') returning id",
    )
    .bind(STALE_CHARACTER)
    .bind(status::PROCESSING)
    .bind(step::FETCHING_ASSETS)
    .fetch_one(&pool)
    .await
    .expect("stale import");

    let fresh: i64 = sqlx::query_scalar(
        "insert into asset_imports (character_id, status, step) values ($1, $2, $3) returning id",
    )
    .bind(STALE_CHARACTER)
    .bind(status::PROCESSING)
    .bind(step::FETCHING_ASSETS)
    .fetch_one(&pool)
    .await
    .expect("fresh import");

    fail_stale_asset_imports(&pool).await.expect("sweep");

    let statuses: Vec<(i64, String)> = sqlx::query_as(
        "select id, status from asset_imports where character_id = $1 order by id",
    )
    .bind(STALE_CHARACTER)
    .fetch_all(&pool)
    .await
    .expect("import rows");
    assert_eq!(
        statuses,
        vec![(stale, status::FAILED.to_owned()), (fresh, status::PROCESSING.to_owned())],
    );
}
