//! Behavior tests for structure resolution against a mock ESI: the public
//! sweep creating stubs, per-character resolution outcomes including the
//! silent 403 (which, faithfully to the legacy connector, costs the
//! token), and the legacy skip-guard precedence quirk.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::structures::{StructureOutcome, sync_public_structures, sync_structure};
use serde_json::json;
use sqlx::PgPool;

const RESOLVER: i64 = 93_000_001;
const OPEN_STRUCTURE: i64 = 1_030_000_001;
const FORBIDDEN_STRUCTURE: i64 = 1_030_000_002;
const SOLAR_SYSTEM: i64 = 30_000_142;
const STRUCTURE_TYPE: i64 = 35_832;
const OWNER: i64 = 98_000_001;

/// Mock ESI: the public structure list plus the detail endpoint, counting
/// detail calls per structure.
fn mock_esi(calls: Arc<Mutex<HashMap<i64, usize>>>) -> Router {
    Router::new()
        .route(
            "/latest/universe/structures/",
            get(|| async {
                (
                    [("x-pages", "1")],
                    Json(json!([OPEN_STRUCTURE, FORBIDDEN_STRUCTURE])),
                )
            }),
        )
        .route(
            "/latest/universe/structures/{structure_id}/",
            get(move |AxumPath(structure_id): AxumPath<i64>| {
                let calls = calls.clone();
                async move {
                    *calls.lock().expect("calls lock").entry(structure_id).or_insert(0) += 1;
                    if structure_id == OPEN_STRUCTURE {
                        Json(json!({
                            "name": "Jita Trade Hub Citadel",
                            "owner_id": OWNER,
                            "solar_system_id": SOLAR_SYSTEM,
                            "type_id": STRUCTURE_TYPE,
                            "position": { "x": 1.0, "y": 2.0, "z": 3.0 },
                        }))
                        .into_response()
                    } else {
                        StatusCode::FORBIDDEN.into_response()
                    }
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

async fn seed_resolver(pool: &PgPool, character_id: i64) {
    sqlx::query("insert into characters (id, name) values ($1, 'Resolver') on conflict (id) do nothing")
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
         values ($1, 'structures-access', 'refresh', 'Bearer', 'owner', $2,
                 now() + interval '20 minutes')",
    )
    .bind(character_id)
    .bind(vec!["esi-structures.read_character.v1".to_owned()])
    .execute(pool)
    .await
    .expect("seed token");
}

async fn setup(structure_ids: &[i64], character_id: i64) -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Idempotent across runs: clear the structures under test (the pivot
    // cascades) and the resolver's state.
    sqlx::query("delete from structures where id = any($1)")
        .bind(structure_ids)
        .execute(&pool)
        .await
        .expect("clean structures");
    seed_resolver(&pool, character_id).await;

    pool
}

/// The SSO is never contacted (the stored token is live), but the clients
/// need an address.
fn sso_stub(base: &str) -> SsoClient {
    SsoClient::new(base, "client", "secret", "http://test/eve/callback")
}

#[tokio::test]
async fn public_sweep_resolves_structures_and_records_failures() {
    let pool = setup(&[OPEN_STRUCTURE, FORBIDDEN_STRUCTURE], RESOLVER).await;

    let calls = Arc::new(Mutex::new(HashMap::new()));
    let esi_url = start_mock(mock_esi(calls.clone())).await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    let stats = sync_public_structures(&pool, &esi, &sso, RESOLVER)
        .await
        .expect("sweep");
    assert_eq!(
        (stats.total, stats.resolved, stats.unresolved, stats.skipped),
        (2, 1, 1, 0),
    );

    // The open structure carries its resolved sheet.
    let (name, owner, type_id, system, fetched): (Option<String>, Option<i64>, Option<i64>, Option<i64>, bool) =
        sqlx::query_as(
            "select name, owner_id, type_id, solarsystem_id, last_fetched_at is not null
             from structures where id = $1",
        )
        .bind(OPEN_STRUCTURE)
        .fetch_one(&pool)
        .await
        .expect("open structure row");
    assert_eq!(name.as_deref(), Some("Jita Trade Hub Citadel"));
    assert_eq!(owner, Some(OWNER));
    assert_eq!(type_id, Some(STRUCTURE_TYPE));
    assert_eq!(system, Some(SOLAR_SYSTEM));
    assert!(fetched);

    // The forbidden one stays an unnamed stub with a recorded failure.
    let name: Option<String> = sqlx::query_scalar("select name from structures where id = $1")
        .bind(FORBIDDEN_STRUCTURE)
        .fetch_one(&pool)
        .await
        .expect("forbidden structure row");
    assert_eq!(name, None);

    let pivots: Vec<(i64, bool)> = sqlx::query_as(
        "select structure_id, could_resolve from character_structure
         where character_id = $1 order by structure_id",
    )
    .bind(RESOLVER)
    .fetch_all(&pool)
    .await
    .expect("pivots");
    assert_eq!(pivots, vec![(OPEN_STRUCTURE, true), (FORBIDDEN_STRUCTURE, false)]);

    // Faithful legacy quirk: the 403 deleted the token, so the character
    // cannot resolve anything until the next SSO login.
    let tokens: i64 = sqlx::query_scalar("select count(*) from esi_tokens where character_id = $1")
        .bind(RESOLVER)
        .fetch_one(&pool)
        .await
        .expect("token count");
    assert_eq!(tokens, 0, "the legacy connector drops the token on 403");

    // A second sweep without a token only creates skips.
    let stats = sync_public_structures(&pool, &esi, &sso, RESOLVER)
        .await
        .expect("tokenless sweep");
    assert_eq!(
        (stats.total, stats.resolved, stats.unresolved, stats.skipped),
        (2, 0, 0, 2),
    );
}

#[tokio::test]
async fn the_skip_guard_spares_only_fresh_known_failures() {
    const GUARD_RESOLVER: i64 = 93_000_002;
    const GUARDED: i64 = 1_030_000_011;
    let pool = setup(&[GUARDED], GUARD_RESOLVER).await;

    let calls = Arc::new(Mutex::new(HashMap::new()));
    // The mock treats every structure as open.
    let esi_url = start_mock(
        Router::new().route(
            "/latest/universe/structures/{structure_id}/",
            get({
                let calls = calls.clone();
                move |AxumPath(structure_id): AxumPath<i64>| {
                    let calls = calls.clone();
                    async move {
                        *calls.lock().expect("calls lock").entry(structure_id).or_insert(0) += 1;
                        Json(json!({
                            "name": "Guarded Fortizar",
                            "owner_id": OWNER,
                            "solar_system_id": SOLAR_SYSTEM,
                            "type_id": STRUCTURE_TYPE,
                        }))
                    }
                }
            }),
        ),
    )
    .await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    // First resolution succeeds.
    let outcome = sync_structure(&pool, &esi, &sso, GUARD_RESOLVER, GUARDED)
        .await
        .expect("first resolution");
    assert_eq!(outcome, StructureOutcome::Resolved);

    // Named, fresh, and previously *resolved*: the legacy guard still
    // refetches (only known failures are spared) — the precedence quirk.
    let outcome = sync_structure(&pool, &esi, &sso, GUARD_RESOLVER, GUARDED)
        .await
        .expect("second resolution");
    assert_eq!(outcome, StructureOutcome::Resolved);
    assert_eq!(calls.lock().expect("calls lock")[&GUARDED], 2);

    // Named, fresh, and a recorded failure: skipped without an ESI call.
    sqlx::query(
        "update character_structure set could_resolve = false
         where character_id = $1 and structure_id = $2",
    )
    .bind(GUARD_RESOLVER)
    .bind(GUARDED)
    .execute(&pool)
    .await
    .expect("record failure");

    let outcome = sync_structure(&pool, &esi, &sso, GUARD_RESOLVER, GUARDED)
        .await
        .expect("guarded attempt");
    assert_eq!(outcome, StructureOutcome::Skipped);
    assert_eq!(calls.lock().expect("calls lock")[&GUARDED], 2, "no new detail call");

    // Once stale (older than a week), even a known failure is retried.
    sqlx::query("update structures set updated_at = now() - interval '8 days' where id = $1")
        .bind(GUARDED)
        .execute(&pool)
        .await
        .expect("age structure");

    let outcome = sync_structure(&pool, &esi, &sso, GUARD_RESOLVER, GUARDED)
        .await
        .expect("stale retry");
    assert_eq!(outcome, StructureOutcome::Resolved);
    assert_eq!(calls.lock().expect("calls lock")[&GUARDED], 3);

    // The retry healed the pivot.
    let resolved: bool = sqlx::query_scalar(
        "select could_resolve from character_structure
         where character_id = $1 and structure_id = $2",
    )
    .bind(GUARD_RESOLVER)
    .bind(GUARDED)
    .fetch_one(&pool)
    .await
    .expect("pivot");
    assert!(resolved);
}
