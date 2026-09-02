//! Behavior tests for the daily alliance sweep against a mock ESI: the
//! legacy GetAlliancesJob → GetAllianceJob → CreateAllianceAction chain
//! ported as one sweep — records upserted with creator character stubs,
//! executor corporations fetched first, per-alliance failures tolerated,
//! reruns updating in place.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use mutamarket::alliances::sync_alliances;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use serde_json::json;
use sqlx::PgPool;

/// Test alliances inside the legacy id-range check.
const FULL_ALLIANCE: i64 = 99_000_101;
const SPARSE_ALLIANCE: i64 = 99_000_102;
const FAILING_ALLIANCE: i64 = 99_000_103;
const FULL_CREATOR: i64 = 93_100_001;
const SPARSE_CREATOR: i64 = 93_100_002;
const EXECUTOR_CORPORATION: i64 = 98_100_001;
const EXECUTOR_CEO: i64 = 93_100_003;

/// Mock ESI: the id list plus per-alliance sheets. `renamed` switches the
/// full alliance's name so a rerun exercises the update path.
fn mock_esi(renamed: Arc<AtomicBool>) -> Router {
    Router::new()
        .route(
            "/latest/alliances/",
            get(|| async { Json(json!([FULL_ALLIANCE, SPARSE_ALLIANCE, FAILING_ALLIANCE])) }),
        )
        .route(
            "/latest/alliances/{alliance_id}/",
            get(move |AxumPath(alliance_id): AxumPath<i64>| {
                let renamed = renamed.clone();
                async move {
                    match alliance_id {
                        FULL_ALLIANCE => Json(json!({
                            "name": if renamed.load(Ordering::Relaxed) {
                                "Goonswarm Reborn"
                            } else {
                                "Goonswarm Federation"
                            },
                            "ticker": "CONDI",
                            "creator_id": FULL_CREATOR,
                            "creator_corporation_id": EXECUTOR_CORPORATION,
                            "date_founded": "2010-06-01T05:36:00Z",
                            "executor_corporation_id": EXECUTOR_CORPORATION,
                            "faction_id": 500001,
                        }))
                        .into_response(),
                        SPARSE_ALLIANCE => Json(json!({
                            // A closed alliance: only the required sheet
                            // fields come back from ESI.
                            "name": "Closed Holdings",
                            "creator_id": SPARSE_CREATOR,
                            "creator_corporation_id": EXECUTOR_CORPORATION,
                        }))
                        .into_response(),
                        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
            }),
        )
        .route(
            "/latest/corporations/{corporation_id}/",
            get(|AxumPath(corporation_id): AxumPath<i64>| async move {
                assert_eq!(corporation_id, EXECUTOR_CORPORATION);
                Json(json!({
                    // A closed corporation: member_count 0 exercises the
                    // legacy truthiness quirk that nulls the stored CEO.
                    "name": "Executor Corp",
                    "ticker": "EXEC",
                    "ceo_id": EXECUTOR_CEO,
                    "creator_id": EXECUTOR_CEO,
                    "member_count": 0,
                    "tax_rate": 0.1,
                }))
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

    sqlx::query("delete from alliances where id = any($1)")
        .bind(vec![FULL_ALLIANCE, SPARSE_ALLIANCE, FAILING_ALLIANCE])
        .execute(&pool)
        .await
        .expect("clean alliances");
    sqlx::query("delete from corporations where id = $1")
        .bind(EXECUTOR_CORPORATION)
        .execute(&pool)
        .await
        .expect("clean executor corporation");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![FULL_CREATOR, SPARSE_CREATOR, EXECUTOR_CEO])
        .execute(&pool)
        .await
        .expect("clean creators");

    pool
}

#[tokio::test]
async fn the_sweep_upserts_alliances_and_tolerates_failures() {
    let pool = setup().await;
    let renamed = Arc::new(AtomicBool::new(false));
    let esi = EsiClient::new(&start_mock(mock_esi(renamed.clone())).await);

    let mut progress_lines = 0usize;
    let stats = sync_alliances(&pool, &esi, |_line| progress_lines += 1)
        .await
        .expect("sweep");
    assert_eq!((stats.total, stats.upserted, stats.failed), (3, 2, 1));
    assert_eq!(progress_lines, 3, "one progress line per alliance");

    type AllianceRow = (
        String,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    );
    let (name, ticker, creator, founded, executor, faction): AllianceRow = sqlx::query_as(
        "select name, ticker, creator_id, date_founded::text,
                executor_corporation_id, faction_id
         from alliances where id = $1",
    )
    .bind(FULL_ALLIANCE)
    .fetch_one(&pool)
    .await
    .expect("full alliance row");
    assert_eq!(name, "Goonswarm Federation");
    assert_eq!(ticker.as_deref(), Some("CONDI"));
    assert_eq!(creator, Some(FULL_CREATOR));
    assert_eq!(founded.as_deref(), Some("2010-06-01 05:36:00+00"));
    assert_eq!(executor, Some(EXECUTOR_CORPORATION));
    assert_eq!(faction, Some(500001));

    // The sparse sheet lands with nulls, like the legacy nullable columns.
    let (name, ticker, executor): (String, Option<String>, Option<i64>) =
        sqlx::query_as("select name, ticker, executor_corporation_id from alliances where id = $1")
            .bind(SPARSE_ALLIANCE)
            .fetch_one(&pool)
            .await
            .expect("sparse alliance row");
    assert_eq!(name, "Closed Holdings");
    assert_eq!(ticker, None);
    assert_eq!(executor, None);

    // The failing sheet left no record.
    let failing: Option<i64> = sqlx::query_scalar("select id from alliances where id = $1")
        .bind(FAILING_ALLIANCE)
        .fetch_optional(&pool)
        .await
        .expect("failing lookup");
    assert_eq!(failing, None);

    // Creators got stub character rows, like Character::insertByIds.
    let creators: Vec<(i64, String)> =
        sqlx::query_as("select id, name from characters where id = any($1) order by id")
            .bind(vec![FULL_CREATOR, SPARSE_CREATOR])
            .fetch_all(&pool)
            .await
            .expect("creator stubs");
    assert_eq!(
        creators,
        vec![
            (FULL_CREATOR, String::new()),
            (SPARSE_CREATOR, String::new())
        ],
    );

    // The executor corporation was fetched like the legacy
    // CreateAllianceAction's GetCorporationJob. member_count 0 nulls the
    // stored CEO (the legacy truthiness quirk); the creator lands with a
    // stub character row.
    let (corp_name, ceo, creator): (String, Option<i64>, Option<i64>) =
        sqlx::query_as("select name, ceo_id, creator_id from corporations where id = $1")
            .bind(EXECUTOR_CORPORATION)
            .fetch_one(&pool)
            .await
            .expect("executor corporation row");
    assert_eq!(corp_name, "Executor Corp");
    assert_eq!(ceo, None);
    assert_eq!(creator, Some(EXECUTOR_CEO));

    // A rerun updates in place (the legacy updateOrCreate).
    renamed.store(true, Ordering::Relaxed);
    let stats = sync_alliances(&pool, &esi, |_line| {})
        .await
        .expect("rerun");
    assert_eq!((stats.total, stats.upserted, stats.failed), (3, 2, 1));
    let (count, name): (i64, String) =
        sqlx::query_as("select count(*) over (), name from alliances where id = $1")
            .bind(FULL_ALLIANCE)
            .fetch_one(&pool)
            .await
            .expect("renamed row");
    assert_eq!(count, 1);
    assert_eq!(name, "Goonswarm Reborn");
}
