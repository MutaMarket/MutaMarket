//! Behavior tests for personal contract ingestion against a mock ESI:
//! upserts into the dedicated character_contracts table (no type or
//! status filtering), acceptor category resolution, item classification
//! with abyssal-only item rows, character-contract unified prices, and
//! the crash-safe item retry derived from items_synced_at.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::contracts::character::{
    character_unified_price, pending_contract_characters, sync_character_contracts,
};
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;

const SELLER: i64 = 95_000_001;
const BUYER_CORPORATION: i64 = 98_000_010;
const BUYER_CORPORATION_CEO: i64 = 95_000_077;
const BUYER_ALLIANCE: i64 = 99_000_123;
const EXCHANGE_CONTRACT: i64 = 950_001;
const AUCTION_CONTRACT: i64 = 950_002;
const COURIER_CONTRACT: i64 = 950_003;
const EXCHANGE_PRICE: f64 = 750_000_000.0;
const PLEX_QUANTITY: i64 = 200;
const PLEX_AVERAGE: f64 = 5_000_000.0;
/// PLEX, the legacy SupportType::PLEX.
const PLEX_TYPE: i64 = 44992;

/// Mock ESI: the character contracts feed, per-contract items (the
/// exchange's items can be told to fail once), and universe names.
fn mock_esi(fail_exchange_items: Arc<AtomicBool>, abyssal_type: i64) -> Router {
    Router::new()
        .route(
            "/latest/characters/{character_id}/contracts/",
            get(move || async move {
                let feed = json!([
                    {
                        "contract_id": EXCHANGE_CONTRACT,
                        "type": "item_exchange",
                        "issuer_id": SELLER,
                        "issuer_corporation_id": 1_000_200,
                        "for_corporation": false,
                        "availability": "public",
                        "status": "outstanding",
                        "title": "abyssal deal",
                        "date_issued": "2026-07-18T10:00:00Z",
                        "date_expired": "2026-08-01T10:00:00Z",
                        "price": EXCHANGE_PRICE,
                        // PHP truthiness: a zero acceptor is no acceptor.
                        "acceptor_id": 0,
                        "assignee_id": 0,
                    },
                    {
                        "contract_id": AUCTION_CONTRACT,
                        "type": "auction",
                        "issuer_id": SELLER,
                        "issuer_corporation_id": 1_000_200,
                        "availability": "personal",
                        "status": "finished",
                        "date_issued": "2026-07-10T10:00:00Z",
                        "date_expired": "2026-07-17T10:00:00Z",
                        "date_accepted": "2026-07-16T09:00:00Z",
                        "date_completed": "2026-07-16T09:00:00Z",
                        "price": 100.0,
                        "buyout": 2_000_000_000.0,
                        "acceptor_id": BUYER_CORPORATION,
                        "assignee_id": BUYER_CORPORATION,
                    },
                    {
                        "contract_id": COURIER_CONTRACT,
                        "type": "courier",
                        "issuer_id": SELLER,
                        "issuer_corporation_id": 1_000_200,
                        "availability": "esoteric",
                        "status": "in_progress",
                        "date_issued": "2026-07-18T08:00:00Z",
                        "date_expired": "2026-07-20T08:00:00Z",
                        "volume": 12_000.0,
                        "acceptor_id": BUYER_ALLIANCE,
                    },
                ]);
                ([("x-pages", "1")], Json(feed))
            }),
        )
        .route(
            "/latest/characters/{character_id}/contracts/{contract_id}/items/",
            get(move |AxumPath((_, contract_id)): AxumPath<(i64, i64)>| {
                let fail_exchange_items = fail_exchange_items.clone();
                async move {
                    match contract_id {
                        EXCHANGE_CONTRACT => {
                            if fail_exchange_items.load(Ordering::SeqCst) {
                                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                            }
                            (
                                [("x-pages", "1")],
                                Json(json!([
                                    // The abyssal module on offer (the
                                    // character endpoint has no item ids).
                                    {
                                        "record_id": 1,
                                        "type_id": abyssal_type,
                                        "quantity": 1,
                                        "is_included": true,
                                        "is_singleton": true,
                                    },
                                    // A non-abyssal extra.
                                    {
                                        "record_id": 2,
                                        "type_id": 34,
                                        "quantity": 500_000,
                                        "is_included": true,
                                        "is_singleton": false,
                                    },
                                    // PLEX asked for on top of the price.
                                    {
                                        "record_id": 3,
                                        "type_id": PLEX_TYPE,
                                        "quantity": PLEX_QUANTITY,
                                        "is_included": false,
                                        "is_singleton": false,
                                    },
                                ])),
                            )
                                .into_response()
                        }
                        AUCTION_CONTRACT => (
                            [("x-pages", "1")],
                            Json(json!([
                                {
                                    "record_id": 4,
                                    "type_id": abyssal_type,
                                    "quantity": 1,
                                    "is_included": true,
                                    "is_singleton": true,
                                },
                            ])),
                        )
                            .into_response(),
                        // The courier's items vanished with the contract.
                        _ => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            }),
        )
        .route(
            "/latest/universe/names/",
            post(|Json(ids): Json<Vec<i64>>| async move {
                let names: Vec<serde_json::Value> = ids
                    .iter()
                    .map(|id| match *id {
                        BUYER_ALLIANCE => json!({
                            "id": id,
                            "name": "Buying Alliance",
                            "category": "alliance",
                        }),
                        _ => json!({
                            "id": id,
                            "name": "Buying Corp",
                            "category": "corporation",
                        }),
                    })
                    .collect();
                Json(names)
            }),
        )
        .route(
            "/latest/corporations/{corporation_id}/",
            get(|AxumPath(corporation_id): AxumPath<i64>| async move {
                assert_eq!(corporation_id, BUYER_CORPORATION);
                Json(json!({
                    "name": "Buying Corp",
                    "ticker": "BUYC",
                    "ceo_id": BUYER_CORPORATION_CEO,
                    "creator_id": BUYER_CORPORATION_CEO,
                    "member_count": 42,
                    "tax_rate": 0.05,
                    "date_founded": "2019-05-01T00:00:00Z",
                }))
            }),
        )
        .route(
            "/latest/alliances/{alliance_id}/",
            get(|AxumPath(alliance_id): AxumPath<i64>| async move {
                assert_eq!(alliance_id, BUYER_ALLIANCE);
                Json(json!({
                    "name": "Buying Alliance",
                    "ticker": "BUY",
                    "creator_id": SELLER,
                    "creator_corporation_id": 1_000_200,
                    "date_founded": "2020-01-01T00:00:00Z",
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

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    // Idempotent across runs: reset this suite's contracts, character and
    // the PLEX average feeding the unified prices.
    sqlx::query("delete from character_contracts where id = any($1)")
        .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT, COURIER_CONTRACT])
        .execute(&pool)
        .await
        .expect("clean contracts");

    sqlx::query(
        "insert into characters (id, name, contracts_fetched_at) values ($1, 'Seller', null)
         on conflict (id) do update set contracts_fetched_at = null",
    )
    .bind(SELLER)
    .execute(&pool)
    .await
    .expect("seed character");

    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(SELLER)
        .execute(&pool)
        .await
        .expect("clean tokens");
    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, 'contracts-access', 'refresh', 'Bearer', 'owner', $2,
                 now() + interval '20 minutes')",
    )
    .bind(SELLER)
    .bind(vec!["esi-contracts.read_character_contracts.v1".to_owned()])
    .execute(&pool)
    .await
    .expect("seed token");

    sqlx::query(
        "insert into regions (id, name) values (10000002, 'The Forge') on conflict (id) do nothing",
    )
    .execute(&pool)
    .await
    .expect("seed region");
    sqlx::query(
        "insert into types (id, name, published) values ($1, 'PLEX', true) on conflict (id) do nothing",
    )
    .bind(PLEX_TYPE)
    .execute(&pool)
    .await
    .expect("seed PLEX type");
    sqlx::query(
        "insert into market_histories
         (type_id, region_id, date, average, highest, lowest, order_count, volume)
         values ($1, 10000002, current_date, $2, $2, $2, 1, 1)
         on conflict (type_id, region_id, date) do update set average = excluded.average",
    )
    .bind(PLEX_TYPE)
    .bind(PLEX_AVERAGE)
    .execute(&pool)
    .await
    .expect("seed PLEX average");

    (pool, reference)
}

fn sso_stub(base: &str) -> SsoClient {
    SsoClient::new(base, "client", "secret", "http://test/eve/callback")
}

#[tokio::test]
async fn character_contracts_sync_stores_classifies_and_retries_items() {
    let (pool, reference) = setup().await;

    let fixtures = common::load_module_fixtures();
    let abyssal_type = fixtures[0].type_id;

    let fail_exchange_items = Arc::new(AtomicBool::new(true));
    let esi_url = start_mock(mock_esi(fail_exchange_items.clone(), abyssal_type)).await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    // The seller holds the scope and was never fetched: pending, first.
    assert!(
        pending_contract_characters(&pool)
            .await
            .expect("pending characters")
            .contains(&SELLER),
    );

    // Historic rows for the back-sync: all three start outstanding.
    sqlx::query("delete from historic_contracts where id = any($1)")
        .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT, COURIER_CONTRACT])
        .execute(&pool)
        .await
        .expect("clean historic rows");
    for contract_id in [EXCHANGE_CONTRACT, AUCTION_CONTRACT, COURIER_CONTRACT] {
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, date_issued)
             values ($1, 'outstanding', 10000002, $2, 'item_exchange', now())",
        )
        .bind(contract_id)
        .bind(SELLER)
        .execute(&pool)
        .await
        .expect("seed historic row");
    }
    sqlx::query("delete from alliances where id = $1")
        .bind(BUYER_ALLIANCE)
        .execute(&pool)
        .await
        .expect("clean alliance");
    sqlx::query("delete from corporations where id = $1")
        .bind(BUYER_CORPORATION)
        .execute(&pool)
        .await
        .expect("clean corporation");
    sqlx::query("delete from characters where id = $1")
        .bind(BUYER_CORPORATION_CEO)
        .execute(&pool)
        .await
        .expect("clean corporation ceo");

    // First sync: the exchange's item fetch fails (500); everything else
    // lands. No type or status filter: the courier is stored too.
    let stats = sync_character_contracts(&pool, &reference, &esi, &sso, SELLER)
        .await
        .expect("first sync");
    assert_eq!(
        (stats.total, stats.items_synced, stats.items_failed),
        (3, 2, 1)
    );

    type ContractRow = (
        i64,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<f64>,
        bool,
    );
    let rows: Vec<ContractRow> = sqlx::query_as(
        "select id, type, availability, status, acceptor_id, acceptor_type, assignee_id,
                    unified_price, items_synced_at is not null
             from character_contracts where id = any($1) order by id",
    )
    .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT, COURIER_CONTRACT])
    .fetch_all(&pool)
    .await
    .expect("contract rows");
    assert_eq!(
        rows,
        vec![
            (
                EXCHANGE_CONTRACT,
                "item_exchange".to_owned(),
                "public".to_owned(),
                // The raw ESI status, stored unparsed like legacy.
                "outstanding".to_owned(),
                None,
                Some("character".to_owned()),
                None,
                // Item sync failed, so the price is still the plain one.
                Some(EXCHANGE_PRICE),
                false,
            ),
            (
                AUCTION_CONTRACT,
                "auction".to_owned(),
                "personal".to_owned(),
                "finished".to_owned(),
                Some(BUYER_CORPORATION),
                Some("corporation".to_owned()),
                Some(BUYER_CORPORATION),
                // No bid is ever known on the character feed: auctions
                // price at zero, like the legacy calculateUnifiedPrice.
                Some(0.0),
                true,
            ),
            (
                COURIER_CONTRACT,
                "courier".to_owned(),
                // Unknown ESI availabilities map to 'unknown'.
                "unknown".to_owned(),
                "in_progress".to_owned(),
                Some(BUYER_ALLIANCE),
                Some("alliance".to_owned()),
                None,
                Some(0.0),
                // Vanished items (404) mark the contract synced anyway.
                true,
            ),
        ],
    );

    // The legacy updateContractStatus back-sync: statuses fold before
    // the write ('finished' -> completed, 'in_progress' -> unknown), and
    // an outstanding contract leaves its historic row untouched.
    let historic: Vec<(i64, String)> =
        sqlx::query_as("select id, status from historic_contracts where id = any($1) order by id")
            .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT, COURIER_CONTRACT])
            .fetch_all(&pool)
            .await
            .expect("historic statuses");
    assert_eq!(
        historic,
        vec![
            (EXCHANGE_CONTRACT, "outstanding".to_owned()),
            (AUCTION_CONTRACT, "completed".to_owned()),
            (COURIER_CONTRACT, "unknown".to_owned()),
        ],
    );

    // The alliance and corporation acceptors got their rows fetched like
    // the legacy CreateContractAcceptorsAction.
    let alliance: (String, Option<String>) =
        sqlx::query_as("select name, ticker from alliances where id = $1")
            .bind(BUYER_ALLIANCE)
            .fetch_one(&pool)
            .await
            .expect("alliance row");
    assert_eq!(
        alliance,
        ("Buying Alliance".to_owned(), Some("BUY".to_owned()))
    );

    let corporation: (
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<f64>,
    ) = sqlx::query_as(
        "select name, ticker, member_count, ceo_id, tax_rate
             from corporations where id = $1",
    )
    .bind(BUYER_CORPORATION)
    .fetch_one(&pool)
    .await
    .expect("corporation row");
    assert_eq!(
        corporation,
        (
            "Buying Corp".to_owned(),
            Some("BUYC".to_owned()),
            Some(42),
            Some(BUYER_CORPORATION_CEO),
            Some(0.05),
        ),
    );

    // Its CEO/creator got a stub character row, like Character::insertByIds.
    let ceo_stub: (String,) = sqlx::query_as("select name from characters where id = $1")
        .bind(BUYER_CORPORATION_CEO)
        .fetch_one(&pool)
        .await
        .expect("ceo stub row");
    assert_eq!(ceo_stub.0, "");

    // The auction's single abyssal item became the only item row so far.
    let auction_items: Vec<(i64, i64)> = sqlx::query_as(
        "select record_id, type_id from character_contract_items
         where character_contract_id = $1 order by record_id",
    )
    .bind(AUCTION_CONTRACT)
    .fetch_all(&pool)
    .await
    .expect("auction items");
    assert_eq!(auction_items, vec![(4, abyssal_type)]);

    let (auction_abyssal, auction_non_abyssal): (i32, i32) = sqlx::query_as(
        "select abyssal_modules_count, non_abyssal_modules_count
         from character_contracts where id = $1",
    )
    .bind(AUCTION_CONTRACT)
    .fetch_one(&pool)
    .await
    .expect("auction counts");
    assert_eq!((auction_abyssal, auction_non_abyssal), (1, 0));

    // The fetch is stamped on the character.
    let fetched: bool =
        sqlx::query_scalar("select contracts_fetched_at is not null from characters where id = $1")
            .bind(SELLER)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert!(fetched);

    // Second sync: the exchange items endpoint recovered; the pending
    // contract (derived from items_synced_at, not an in-memory diff) is
    // retried and classified.
    fail_exchange_items.store(false, Ordering::SeqCst);
    let stats = sync_character_contracts(&pool, &reference, &esi, &sso, SELLER)
        .await
        .expect("second sync");
    assert_eq!(
        (stats.total, stats.items_synced, stats.items_failed),
        (3, 1, 0)
    );

    let (asking, plex_count, abyssal, non_abyssal, unified, synced): (
        bool,
        i32,
        i32,
        i32,
        Option<f64>,
        bool,
    ) = sqlx::query_as(
        "select asking_for_items, plex_count, abyssal_modules_count, non_abyssal_modules_count,
                unified_price, items_synced_at is not null
         from character_contracts where id = $1",
    )
    .bind(EXCHANGE_CONTRACT)
    .fetch_one(&pool)
    .await
    .expect("exchange row");
    assert!(asking);
    assert_eq!(plex_count, PLEX_QUANTITY as i32);
    assert_eq!((abyssal, non_abyssal), (1, 2));
    assert_eq!(
        unified,
        Some(EXCHANGE_PRICE + PLEX_AVERAGE * PLEX_QUANTITY as f64)
    );
    assert!(synced);

    let exchange_items: Vec<(i64, i64)> = sqlx::query_as(
        "select record_id, type_id from character_contract_items
         where character_contract_id = $1 order by record_id",
    )
    .bind(EXCHANGE_CONTRACT)
    .fetch_all(&pool)
    .await
    .expect("exchange items");
    assert_eq!(
        exchange_items,
        vec![(1, abyssal_type)],
        "only the abyssal module is a row"
    );

    // A third sync is idempotent: everything already synced, upserts only.
    let stats = sync_character_contracts(&pool, &reference, &esi, &sso, SELLER)
        .await
        .expect("third sync");
    assert_eq!(
        (stats.total, stats.items_synced, stats.items_failed),
        (3, 0, 0)
    );

    // Legacy quirk, ported faithfully: the refetch upsert recomputes the
    // unified price from a fresh model that knows no plex count, so the
    // PLEX component is clobbered back out until an item sync would run
    // again (which it does not, once synced).
    let clobbered: Option<f64> =
        sqlx::query_scalar("select unified_price from character_contracts where id = $1")
            .bind(EXCHANGE_CONTRACT)
            .fetch_one(&pool)
            .await
            .expect("exchange row");
    assert_eq!(clobbered, Some(EXCHANGE_PRICE));
}

#[test]
fn character_unified_prices_follow_the_legacy_model() {
    // Auctions count only their highest bid — no price fallback.
    assert_eq!(
        character_unified_price("auction", Some(500.0), None, 0, None),
        0.0
    );
    assert_eq!(
        character_unified_price("auction", Some(500.0), Some(900.0), 0, None),
        900.0
    );
    // Exchanges add the asked-for PLEX at the latest average.
    assert_eq!(
        character_unified_price("item_exchange", Some(100.0), None, 3, Some(10.0)),
        130.0,
    );
    assert_eq!(
        character_unified_price("item_exchange", None, None, 2, None),
        0.0
    );
    // Everything else prices at zero (the legacy match default).
    assert_eq!(
        character_unified_price("courier", Some(1_000.0), Some(5.0), 1, Some(2.0)),
        0.0
    );
}
