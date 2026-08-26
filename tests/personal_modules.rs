//! Behavior tests for the personal modules data and its import trigger:
//! the guest handling, the panel payload states (including the missing
//! scope link), the store action's scope guard, and the full import round
//! trip against a mock ESI ending in an owned module in the payload.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

/// Each test owns one character so parallel tests never share state.
const NO_SCOPE_CHARACTER: i64 = 96_100_001;
const IMPORT_CHARACTER: i64 = 96_100_002;
const RENDER_CHARACTER: i64 = 96_100_003;

const ACCESS_TOKEN: &str = "personal-assets-access";
/// Jita IV-4, the NPC station the imported module sits in.
const STATION: i64 = 60_003_760;
/// The read-assets scope string, as stored on tokens.
const READ_ASSETS: &str = "esi-assets.read_assets.v1";

/// The spawned background import must finish within this window.
const IMPORT_TIMEOUT: Duration = Duration::from_secs(10);

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> =
        value.as_object().expect("a JSON object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");

    (pool, ReferenceData::from_tables(tables))
}

fn app(pool: &PgPool, reference: ReferenceData, esi_url: &str) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new(esi_url),
        SsoClient::new("http://127.0.0.1:9", "client", "secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator_stub(),
        Arc::new(reference),
        None,
    )
}

/// A fresh user with one character and a session; previous state of that
/// character is cleaned for idempotency.
async fn seed_user(pool: &PgPool, character_id: i64, scopes: &[&str]) -> (i64, String) {
    sqlx::query("update characters set latest_asset_import_id = null where id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("unlink import");
    for table in ["asset_imports", "assets", "esi_tokens"] {
        sqlx::query(&format!("delete from {table} where character_id = $1"))
            .bind(character_id)
            .execute(pool)
            .await
            .expect("clean character state");
    }
    sqlx::query("delete from characters where id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("clean character");

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Personal Pilot') returning id")
            .fetch_one(pool)
            .await
            .expect("create user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Personal Pilot', $2)")
        .bind(character_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("create character");

    if !scopes.is_empty() {
        sqlx::query(
            "insert into esi_tokens
             (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
              expires_at)
             values ($1, $2, 'refresh', 'Bearer', 'owner', $3, now() + interval '20 minutes')",
        )
        .bind(character_id)
        .bind(ACCESS_TOKEN)
        .bind(scopes.iter().map(|scope| scope.to_string()).collect::<Vec<_>>())
        .execute(pool)
        .await
        .expect("seed token");
    }

    let session = create_session(pool, user_id, Some(character_id))
        .await
        .expect("create session");

    (user_id, session)
}

async fn send(app: &Router, method: Method, path: &str, session: Option<&str>) -> (StatusCode, String, String) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }

    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("infallible");

    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response.into_body().collect().await.expect("body").to_bytes();

    (status, location, String::from_utf8_lossy(&body).into_owned())
}

/// Mock ESI: one abyssal module loose in a station hangar, its player
/// name, and its rolled dogma. Authenticated routes check the bearer.
fn mock_esi(module: serde_json::Value) -> Router {
    let module_item = module["item_id"].as_i64().expect("item id");
    let module_type = module["type_id"].as_i64().expect("type id");

    let bearer_ok = |headers: &HeaderMap| {
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            == Some(&format!("Bearer {ACCESS_TOKEN}"))
    };

    Router::new()
        .route(
            "/latest/characters/{character_id}/assets/",
            get(move |headers: HeaderMap| async move {
                if !bearer_ok(&headers) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                let feed = json!([
                    {
                        "item_id": module_item, "type_id": module_type,
                        "location_id": STATION, "location_type": "station",
                        "location_flag": "Hangar", "quantity": 1, "is_singleton": true,
                    },
                    {
                        "item_id": 9_101, "type_id": 34, "location_id": STATION,
                        "location_type": "station", "location_flag": "Hangar",
                        "quantity": 500_000, "is_singleton": false,
                    },
                ]);
                ([("x-pages", "1")], Json(feed)).into_response()
            }),
        )
        .route(
            "/latest/characters/{character_id}/assets/names/",
            post(move |headers: HeaderMap, Json(ids): Json<Vec<i64>>| async move {
                if !bearer_ok(&headers) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                let names: Vec<serde_json::Value> = ids
                    .iter()
                    .map(|id| json!({ "item_id": id, "name": "None" }))
                    .collect();
                Json(names).into_response()
            }),
        )
        .route(
            "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
            get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
                let module = module.clone();
                async move {
                    if module["type_id"] == json!(type_id) && module["item_id"] == json!(item_id) {
                        return Json(module["dogma"].clone()).into_response();
                    }
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
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

#[tokio::test]
async fn guests_are_redirected_to_login() {
    let (pool, reference) = setup().await;
    let app = app(&pool, reference, "http://127.0.0.1:9");

    let (status, location, _) = send(&app, Method::POST, "/personal/modules", None).await;
    assert!(status.is_redirection(), "guest POST must redirect, got {status}");
    assert_eq!(location, "/login");
}

#[tokio::test]
async fn page_data_carries_the_scope_state_and_the_guard_blocks_imports() {
    let (pool, reference) = setup().await;
    let (_, session) = seed_user(&pool, NO_SCOPE_CHARACTER, &[]).await;
    let app = app(&pool, reference, "http://127.0.0.1:9");

    // Without the Read Assets scope the payload carries the grant CTA
    // target and no import state, with the exact key set the frontend
    // consumes.
    let (status, _, body) = send(&app, Method::GET, "/api/personal/page", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        sorted_keys(&page),
        [
            "asset_import",
            "estimated_value_total",
            "grant_scope_url",
            "has_assets_scope",
            "modules_count",
            "user_id",
        ],
    );
    assert_eq!(page["has_assets_scope"], json!(false));
    assert_eq!(page["grant_scope_url"], json!("/eve?scopes=esi-assets.read_assets.v1"));
    assert_eq!(page["asset_import"], serde_json::Value::Null);

    // The store action redirects back without dispatching anything, like
    // the legacy scope guard (its notification is not ported).
    let (status, location, _) = send(&app, Method::POST, "/personal/modules", Some(&session)).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/personal/modules");

    let imports: i64 =
        sqlx::query_scalar("select count(*) from asset_imports where character_id = $1")
            .bind(NO_SCOPE_CHARACTER)
            .fetch_one(&pool)
            .await
            .expect("count imports");
    assert_eq!(imports, 0, "no import may start without the scope");
}

#[tokio::test]
async fn page_data_reports_the_granted_scope() {
    let (pool, reference) = setup().await;
    let (_, session) = seed_user(&pool, RENDER_CHARACTER, &[READ_ASSETS]).await;
    let app = app(&pool, reference, "http://127.0.0.1:9");

    let (status, _, body) = send(&app, Method::GET, "/api/personal/page", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["has_assets_scope"], json!(true));
    assert_eq!(page["asset_import"], serde_json::Value::Null);

    let (status, _, body) = send(&app, Method::GET, "/api/personal/modules", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    let entries: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(entries, json!([]), "no owned modules yet");
}

#[tokio::test]
async fn starting_an_import_ingests_the_assets_and_shows_the_owned_module() {
    let (pool, reference) = setup().await;
    let (_, session) = seed_user(&pool, IMPORT_CHARACTER, &[READ_ASSETS]).await;

    // The third fixture file keeps this test's module distinct from the
    // ones the other suites ingest.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[2];
    let module = &fixture.modules[0];
    let esi_url = start_mock(mock_esi(dogma_payload(fixture.type_id, module))).await;

    let app = app(&pool, reference, &esi_url);

    let (status, location, _) = send(&app, Method::POST, "/personal/modules", Some(&session)).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/personal/modules", "back() falls back to the page");

    // The import runs in the background; wait for the state machine to
    // complete like the socket-driven panel would.
    let deadline = tokio::time::Instant::now() + IMPORT_TIMEOUT;
    let import = loop {
        let import: Option<(String, i32, i32)> = sqlx::query_as(
            "select status, abyssal_modules_count, abyssal_modules_imported_count
             from asset_imports where character_id = $1
             order by id desc limit 1",
        )
        .bind(IMPORT_CHARACTER)
        .fetch_optional(&pool)
        .await
        .expect("read import");

        if let Some(import) = &import
            && (import.0 == "completed" || import.0 == "failed")
        {
            break import.clone();
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "import did not finish in time: {import:?}",
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    };

    assert_eq!(
        (import.0.as_str(), import.1, import.2),
        ("completed", 1, 1),
        "the single abyssal module imports",
    );

    // The location resolves the hosting station via the parent chain: the
    // module lies loose in the hangar, so the station itself names the
    // row (legacy AssetResource fallback order), with the humanized flag
    // label and the one-based location index.
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

    // The JSON endpoints serve the same state to the frontend: the
    // completed panel data and the owned module with its location.
    let (status, _, body) = send(&app, Method::GET, "/api/personal/page", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        sorted_keys(&page),
        [
            "asset_import",
            "estimated_value_total",
            "grant_scope_url",
            "has_assets_scope",
            "modules_count",
            "user_id",
        ],
    );
    assert_eq!(page["has_assets_scope"], json!(true));
    assert_eq!(
        sorted_keys(&page["asset_import"]),
        [
            "abyssal_modules_count",
            "abyssal_modules_failed_count",
            "abyssal_modules_imported_count",
            "assets_corporation_count",
            "assets_count",
            "character_id",
            "id",
            "status",
            "step",
            "updated_seconds_ago",
        ],
    );
    assert_eq!(page["asset_import"]["status"], json!("completed"));
    assert_eq!(page["asset_import"]["character_id"], json!(IMPORT_CHARACTER));
    assert_eq!(page["asset_import"]["abyssal_modules_imported_count"], json!(1));

    let (status, _, body) = send(&app, Method::GET, "/api/personal/modules", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    let entries: serde_json::Value = serde_json::from_str(&body).expect("json");
    let entries = entries.as_array().expect("entry array");
    assert_eq!(entries.len(), 1, "the imported module is owned");
    assert_eq!(sorted_keys(&entries[0]), ["location", "module"]);
    assert_eq!(entries[0]["module"]["id"], json!(module.module_id));
    assert_eq!(
        sorted_keys(&entries[0]["location"]),
        [
            "corporation_id",
            "location_flag",
            "location_id",
            "location_index",
            "location_type",
            "parent_name",
            "parent_slug",
            "parent_type_id",
            "station",
        ],
    );
    assert_eq!(
        entries[0]["location"]["parent_name"],
        json!("Jita IV - Moon 4 - Caldari Navy Assembly Plant"),
    );
    assert_eq!(entries[0]["location"]["location_flag"], json!("Hangar"));
    assert_eq!(
        entries[0]["location"]["parent_slug"],
        json!(format!("jita-iv-moon-4-caldari-navy-assembly-plant-{STATION}")),
        "the slug feeds the legacy locations route",
    );

    // Guests get the fetch-shaped 401 (documented divergence from the
    // page routes' login redirect).
    let (status, _, body) = send(&app, Method::GET, "/api/personal/modules", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(error["message"], json!("Unauthenticated."));
}
