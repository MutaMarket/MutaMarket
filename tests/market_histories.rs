//! Behavior tests for the daily market-history sweep against a mock ESI:
//! the legacy `GetMarketHistoriesCommand` type set (mutaplasmids,
//! published source types, PLEX), the newest-day-only storage of the
//! per-type job, PLEX keeping its full history, and per-type failure
//! tolerance.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use mutamarket::contracts::{
    FORGE_REGION_ID, PLEX_TYPE_ID, market_history_type_ids, sync_market_history_set,
};
use mutamarket::db;
use mutamarket::esi::EsiClient;
use serde_json::json;
use sqlx::PgPool;

/// Test-only reference rows, high ids clear of the real SDE ranges.
const MUTAPLASMID: i64 = 950_000_001;
const OUTPUT_TYPE: i64 = 950_000_002;
const SOURCE_TYPE: i64 = 950_000_003;
const UNPUBLISHED_SOURCE_TYPE: i64 = 950_000_004;
const FAILING_MUTAPLASMID: i64 = 950_000_005;
const INPUT_ROW_BASE: i64 = 950_000_100;

#[derive(serde::Deserialize)]
struct HistoryQuery {
    type_id: i64,
}

/// Mock ESI: The Forge's history endpoint keyed by type_id. The source
/// type answers two days (only the newest may be stored), the PLEX type
/// two days (all stored), the mutaplasmid an empty history, the failing
/// mutaplasmid a 500.
fn mock_esi() -> Router {
    Router::new().route(
        "/latest/markets/{region_id}/history/",
        get(|Query(query): Query<HistoryQuery>| async move {
            match query.type_id {
                SOURCE_TYPE => Json(json!([
                    {"date": "2026-08-26", "average": 100.0, "highest": 120.0,
                     "lowest": 90.0, "order_count": 5, "volume": 11},
                    {"date": "2026-08-27", "average": 200.0, "highest": 220.0,
                     "lowest": 180.0, "order_count": 7, "volume": 13},
                ]))
                .into_response(),
                PLEX_TYPE_ID => Json(json!([
                    {"date": "2026-08-26", "average": 4_000_000.0, "highest": 4_100_000.0,
                     "lowest": 3_900_000.0, "order_count": 100, "volume": 500},
                    {"date": "2026-08-27", "average": 4_500_000.0, "highest": 4_600_000.0,
                     "lowest": 4_400_000.0, "order_count": 110, "volume": 600},
                ]))
                .into_response(),
                MUTAPLASMID => Json(json!([])).into_response(),
                _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    sqlx::query(
        "insert into regions (id, name) values ($1, 'The Forge') on conflict (id) do nothing",
    )
    .bind(FORGE_REGION_ID)
    .execute(&pool)
    .await
    .expect("seed region");
    sqlx::query("insert into types (id, name, published) values ($1, 'PLEX', true) on conflict (id) do nothing")
        .bind(PLEX_TYPE_ID)
        .execute(&pool)
        .await
        .expect("seed PLEX type");

    // Idempotent across runs: drop the test reference rows and any
    // history they produced.
    let type_ids = vec![
        MUTAPLASMID,
        OUTPUT_TYPE,
        SOURCE_TYPE,
        UNPUBLISHED_SOURCE_TYPE,
        FAILING_MUTAPLASMID,
    ];
    sqlx::query("delete from market_histories where type_id = any($1) or type_id = $2")
        .bind(&type_ids)
        .bind(PLEX_TYPE_ID)
        .execute(&pool)
        .await
        .expect("clean histories");
    sqlx::query("delete from mutaplasmid_input_types where id >= $1 and id < $1 + 10")
        .bind(INPUT_ROW_BASE)
        .execute(&pool)
        .await
        .expect("clean input types");
    sqlx::query("delete from mutaplasmids where id = any($1)")
        .bind(vec![MUTAPLASMID, FAILING_MUTAPLASMID])
        .execute(&pool)
        .await
        .expect("clean mutaplasmids");
    sqlx::query("delete from types where id = any($1)")
        .bind(&type_ids)
        .execute(&pool)
        .await
        .expect("clean types");

    for (type_id, name, published) in [
        (OUTPUT_TYPE, "Abyssal Sweep Output", true),
        (SOURCE_TYPE, "Sweep Source I", true),
        (UNPUBLISHED_SOURCE_TYPE, "Sweep Source Unpublished", false),
    ] {
        sqlx::query("insert into types (id, name, published) values ($1, $2, $3)")
            .bind(type_id)
            .bind(name)
            .bind(published)
            .execute(&pool)
            .await
            .expect("seed type");
    }
    for (mutaplasmid_id, name) in [
        (MUTAPLASMID, "Sweep Mutaplasmid"),
        (FAILING_MUTAPLASMID, "Failing Mutaplasmid"),
    ] {
        sqlx::query("insert into mutaplasmids (id, name, output_type_id) values ($1, $2, $3)")
            .bind(mutaplasmid_id)
            .bind(name)
            .bind(OUTPUT_TYPE)
            .execute(&pool)
            .await
            .expect("seed mutaplasmid");
    }
    for (offset, type_id) in [SOURCE_TYPE, UNPUBLISHED_SOURCE_TYPE]
        .into_iter()
        .enumerate()
    {
        sqlx::query(
            "insert into mutaplasmid_input_types (id, mutaplasmid_id, type_id)
             values ($1, $2, $3)",
        )
        .bind(INPUT_ROW_BASE + offset as i64)
        .bind(MUTAPLASMID)
        .bind(type_id)
        .execute(&pool)
        .await
        .expect("seed input type");
    }

    pool
}

#[tokio::test]
async fn the_sweep_covers_the_legacy_type_set_and_stores_latest_days() {
    let pool = setup().await;
    let esi = EsiClient::new(&start_mock(mock_esi()).await);

    // The legacy dispatch order: mutaplasmids, published source types,
    // then the SupportType (PLEX). Unpublished source types are skipped.
    let type_ids = market_history_type_ids(&pool).await.expect("type set");
    let ours: Vec<i64> = type_ids
        .iter()
        .copied()
        .filter(|id| (MUTAPLASMID..=FAILING_MUTAPLASMID).contains(id) || *id == PLEX_TYPE_ID)
        .collect();
    assert_eq!(
        ours,
        [MUTAPLASMID, FAILING_MUTAPLASMID, SOURCE_TYPE, PLEX_TYPE_ID]
    );
    assert_eq!(
        *type_ids.last().expect("non-empty"),
        PLEX_TYPE_ID,
        "PLEX closes the sweep"
    );
    assert!(
        !type_ids.contains(&UNPUBLISHED_SOURCE_TYPE),
        "unpublished source types stay out, like the legacy published() scope",
    );

    // The sweep itself runs over just the seeded ids (the shared test
    // database carries unrelated reference data other suites seed).
    let mut progress_lines = 0usize;
    let stats = sync_market_history_set(&pool, &esi, &ours, |_line| progress_lines += 1).await;
    assert_eq!(stats.types, ours.len());
    // PLEX stores both days, the source type only its newest; the empty
    // history and the 500 are tolerated per type, like the legacy job.
    assert_eq!((stats.days, stats.empty, stats.failed), (3, 1, 1));
    assert_eq!(progress_lines, stats.types, "one progress line per type");

    let source_rows: Vec<(String, f64, f64, f64, i64, i64)> = sqlx::query_as(
        "select date::text, average, highest, lowest, order_count, volume
         from market_histories where type_id = $1 order by date",
    )
    .bind(SOURCE_TYPE)
    .fetch_all(&pool)
    .await
    .expect("source rows");
    assert_eq!(
        source_rows,
        vec![("2026-08-27".to_owned(), 200.0, 220.0, 180.0, 7, 13)],
        "only the newest day is stored, like the legacy ProcessMarketHistory",
    );

    let plex_dates: Vec<String> = sqlx::query_scalar(
        "select date::text from market_histories
         where type_id = $1 and region_id = $2 order by date",
    )
    .bind(PLEX_TYPE_ID)
    .bind(FORGE_REGION_ID)
    .fetch_all(&pool)
    .await
    .expect("plex rows");
    assert_eq!(
        plex_dates,
        ["2026-08-26", "2026-08-27"],
        "PLEX keeps its full history"
    );

    let empty_and_failed: i64 =
        sqlx::query_scalar("select count(*) from market_histories where type_id = any($1)")
            .bind(vec![MUTAPLASMID, FAILING_MUTAPLASMID])
            .fetch_one(&pool)
            .await
            .expect("empty/failed count");
    assert_eq!(
        empty_and_failed, 0,
        "no rows for empty or failing histories"
    );

    // A second sweep upserts the same days without duplicating rows.
    let stats = sync_market_history_set(&pool, &esi, &ours, |_line| {}).await;
    assert_eq!((stats.days, stats.empty, stats.failed), (3, 1, 1));
    let source_count: i64 =
        sqlx::query_scalar("select count(*) from market_histories where type_id = $1")
            .bind(SOURCE_TYPE)
            .fetch_one(&pool)
            .await
            .expect("source count");
    assert_eq!(source_count, 1);
}
