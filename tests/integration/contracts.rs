//! Behavior tests for public contract ingestion against a mock ESI: region
//! sync with classification and module linking, unified prices including
//! asked-for PLEX, auction bids, invalidation, and the contract emitted in
//! the module JSON with the legacy resource key set.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use http_body_util::BodyExt;
use mutamarket::contracts::{
    FORGE_REGION_ID, plex_average, sync_auction_bids, sync_plex_market_history, sync_region,
};
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use tower::ServiceExt;

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

const EXCHANGE_CONTRACT: i64 = 900_001;
const AUCTION_CONTRACT: i64 = 900_002;
const ISSUER: i64 = 91_000_001;
const PLEX_QUANTITY: i64 = 500;
const PLEX_AVERAGE: f64 = 5_000_000.0;
const EXCHANGE_PRICE: f64 = 1_000_000_000.0;
const AUCTION_START_PRICE: f64 = 2_000_000_000.0;
const HIGHEST_BID: f64 = 3_000_000_000.0;

/// Mock ESI: contracts feed (second pass drops the item exchange, third
/// pass also the auction), items, bids, market history and dynamic items
/// for the two fixture modules.
fn mock_esi(
    second_pass: Arc<AtomicBool>,
    third_pass: Arc<AtomicBool>,
    fail_dynamic: Arc<AtomicBool>,
    exchange_module: serde_json::Value,
    auction_module: serde_json::Value,
) -> Router {
    let exchange_item = exchange_module["item_id"].as_i64().expect("item id");
    let auction_item = auction_module["item_id"].as_i64().expect("item id");
    let exchange_type = exchange_module["type_id"].as_i64().expect("type id");
    let auction_type = auction_module["type_id"].as_i64().expect("type id");

    let items_pass = third_pass.clone();
    let feed_third_pass = third_pass.clone();

    Router::new()
        .route(
            "/latest/contracts/public/{region_id}/",
            get(move || {
                let second_pass = second_pass.clone();
                let third_pass = feed_third_pass.clone();
                async move {
                    let exchange = json!({
                        "contract_id": EXCHANGE_CONTRACT,
                        "type": "item_exchange",
                        "issuer_id": ISSUER,
                        "issuer_corporation_id": 1_000_100,
                        "date_issued": "2026-07-18T10:00:00Z",
                        "date_expired": "2026-08-01T10:00:00Z",
                        "price": EXCHANGE_PRICE,
                        "title": "juicy roll",
                        "start_location_id": 60003760i64,
                    });
                    let auction = json!({
                        "contract_id": AUCTION_CONTRACT,
                        "type": "auction",
                        "issuer_id": ISSUER,
                        "issuer_corporation_id": 1_000_100,
                        "date_issued": "2026-07-18T09:00:00Z",
                        "date_expired": "2026-07-25T09:00:00Z",
                        "price": AUCTION_START_PRICE,
                        "buyout": 9_000_000_000.0,
                    });
                    let courier = json!({
                        "contract_id": 900_003,
                        "type": "courier",
                        "issuer_id": ISSUER,
                        "issuer_corporation_id": 1_000_100,
                        "date_issued": "2026-07-18T08:00:00Z",
                        "date_expired": "2026-07-20T08:00:00Z",
                    });

                    let feed = if third_pass.load(Ordering::SeqCst) {
                        json!([courier])
                    } else if second_pass.load(Ordering::SeqCst) {
                        json!([auction, courier])
                    } else {
                        json!([exchange, auction, courier])
                    };

                    ([("x-pages", "1")], Json(feed)).into_response()
                }
            }),
        )
        .route(
            "/latest/contracts/public/items/{contract_id}/",
            get(move |AxumPath(contract_id): AxumPath<i64>| {
                let items_pass = items_pass.clone();
                async move {
                // On the third pass the vanished auction answers with
                // the accepted-by-player error, the signal the
                // invalidation status probe reads.
                if items_pass.load(Ordering::SeqCst) && contract_id == AUCTION_CONTRACT {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({ "error": "Contract accepted by player" })),
                    )
                        .into_response();
                }
                let items = match contract_id {
                    EXCHANGE_CONTRACT => json!([
                        {
                            "record_id": 1,
                            "type_id": exchange_type,
                            "item_id": exchange_item,
                            "quantity": 1,
                            "is_included": true,
                        },
                        // A non-abyssal included item.
                        {
                            "record_id": 2,
                            "type_id": 34,
                            "quantity": 1_000_000,
                            "is_included": true,
                        },
                        // PLEX asked for as part of the payment.
                        {
                            "record_id": 3,
                            "type_id": 44992,
                            "quantity": PLEX_QUANTITY,
                            "is_included": false,
                        },
                    ]),
                    AUCTION_CONTRACT => json!([
                        {
                            "record_id": 4,
                            "type_id": auction_type,
                            "item_id": auction_item,
                            "quantity": 1,
                            "is_included": true,
                        },
                    ]),
                    _ => return StatusCode::NOT_FOUND.into_response(),
                };

                ([("x-pages", "1")], Json(items)).into_response()
                }
            }),
        )
        .route(
            "/latest/contracts/public/bids/{contract_id}/",
            get(|AxumPath(contract_id): AxumPath<i64>| async move {
                if contract_id == AUCTION_CONTRACT {
                    Json(json!([
                        { "bid_id": 1, "amount": 2_500_000_000.0, "date_bid": "2026-07-18T11:00:00Z" },
                        { "bid_id": 2, "amount": HIGHEST_BID, "date_bid": "2026-07-18T12:00:00Z" },
                    ]))
                    .into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
        .route(
            "/latest/markets/{region_id}/history/",
            get(|| async {
                Json(json!([
                    { "date": "2026-07-16", "average": 4_900_000.0, "highest": 5_000_000.0,
                      "lowest": 4_800_000.0, "order_count": 100, "volume": 5000 },
                    { "date": "2026-07-17", "average": PLEX_AVERAGE, "highest": 5_100_000.0,
                      "lowest": 4_900_000.0, "order_count": 120, "volume": 6000 },
                ]))
            }),
        )
        .route(
            "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
            get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
                let exchange_module = exchange_module.clone();
                let auction_module = auction_module.clone();
                let fail_dynamic = fail_dynamic.clone();
                async move {
                    if fail_dynamic.load(Ordering::SeqCst) {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                    for module in [&exchange_module, &auction_module] {
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

/// ESI answers some contracts' items with a 200 and no body at all
/// (seen on tranquility for expired contracts); the client reads that
/// as no items instead of a decode failure.
#[tokio::test]
async fn an_empty_items_body_reads_as_no_items() {
    let router = Router::new().route(
        "/latest/contracts/public/items/{contract_id}/",
        get(|| async { ([("x-pages", "1")], "").into_response() }),
    );
    let esi_url = start_mock(router).await;
    let esi = EsiClient::new(&esi_url);

    let (items, pages) = esi
        .public_contract_items(1, 1)
        .await
        .expect("an empty body is not an error");
    assert!(items.is_empty());
    assert_eq!(pages, 1);
}

#[tokio::test]
async fn contracts_sync_ingests_classifies_and_links_modules() {
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

    sqlx::query(
        "insert into regions (id, name) values ($1, 'The Forge') on conflict (id) do nothing",
    )
    .bind(FORGE_REGION_ID)
    .execute(&pool)
    .await
    .expect("seed region");

    // PLEX exists in the full SDE but not in the filtered fixture subset.
    sqlx::query(
        "insert into types (id, name, published) values (44992, 'PLEX', true)
         on conflict (id) do nothing",
    )
    .execute(&pool)
    .await
    .expect("seed PLEX type");

    // Idempotency across runs and suites: the invalidation count compares
    // against every known contract of the region, so start clean.
    sqlx::query("delete from contracts where region_id = $1")
        .bind(FORGE_REGION_ID)
        .execute(&pool)
        .await
        .expect("clean contracts");
    sqlx::query("delete from historic_contracts where id = any($1)")
        .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT])
        .execute(&pool)
        .await
        .expect("clean historic contracts");
    // Unified prices read the newest PLEX daily average, so a row this
    // suite did not seed decides the answer. `market_histories` seeds
    // later dates for the same type against the same database, and test
    // binaries run one after another, so whichever ran last wins unless
    // this starts from an empty PLEX history.
    sqlx::query("delete from market_histories where type_id = 44992")
        .execute(&pool)
        .await
        .expect("clean PLEX history");

    let fixtures = common::load_module_fixtures();
    let exchange_fixture = fixtures
        .iter()
        .find(|f| f.type_id == 47736)
        .expect("fixture");
    let auction_fixture = fixtures
        .iter()
        .find(|f| f.type_id == 47740)
        .expect("fixture");
    let exchange_module = &exchange_fixture.modules[0];
    let auction_module = &auction_fixture.modules[0];

    let second_pass = Arc::new(AtomicBool::new(false));
    let third_pass = Arc::new(AtomicBool::new(false));
    let fail_dynamic = Arc::new(AtomicBool::new(false));
    let esi_url = start_mock(mock_esi(
        second_pass.clone(),
        third_pass.clone(),
        fail_dynamic.clone(),
        dogma_payload(exchange_fixture.type_id, exchange_module),
        dogma_payload(auction_fixture.type_id, auction_module),
    ))
    .await;
    let esi = EsiClient::new(&esi_url);

    // Market history first: the PLEX average feeds unified prices.
    let days = sync_plex_market_history(&pool, &esi)
        .await
        .expect("plex history");
    assert_eq!(days, 2);
    assert_eq!(
        plex_average(&pool).await.expect("average"),
        Some(PLEX_AVERAGE)
    );

    // First sync: the courier is filtered, both relevant contracts land,
    // items are fetched and classified, modules imported and linked.
    let phases = std::sync::Mutex::new(Vec::<String>::new());
    let report = |phase: &str| phases.lock().expect("phases").push(phase.to_owned());
    let stats = sync_region(
        &pool,
        &reference,
        &esi,
        &estimator_stub(),
        FORGE_REGION_ID,
        &report,
    )
    .await
    .expect("sync region");
    assert_eq!(
        (stats.total, stats.relevant, stats.new, stats.invalidated),
        (3, 2, 2, 0)
    );
    // The admin console's progress line advances through the phases, so
    // a region running its item fetches for minutes never reads frozen.
    let phases = phases.into_inner().expect("phases");
    assert_eq!(
        phases,
        [
            "fetching contracts",
            "3 contracts, 2 relevant: saving 2 new",
            "items 0/2 contracts synced",
            "items 1/2 contracts synced",
            "items 2/2 contracts synced",
        ],
    );

    let (asking, plex_count, abyssal, non_abyssal, unified): (bool, i32, i32, i32, Option<f64>) =
        sqlx::query_as(
            "select asking_for_items, plex_count, abyssal_modules_count,
                    non_abyssal_modules_count, unified_price
             from contracts where id = $1",
        )
        .bind(EXCHANGE_CONTRACT)
        .fetch_one(&pool)
        .await
        .expect("exchange contract");
    assert!(asking, "PLEX is asked for");
    assert_eq!(plex_count, PLEX_QUANTITY as i32);
    assert_eq!(abyssal, 1);
    assert_eq!(non_abyssal, 2, "the tritanium stack and the asked-for PLEX");
    assert_eq!(
        unified,
        Some(EXCHANGE_PRICE + PLEX_AVERAGE * PLEX_QUANTITY as f64),
        "item exchange unified price includes the asked-for PLEX",
    );

    let latest: Option<i64> =
        sqlx::query_scalar("select latest_contract_id from modules where id = $1")
            .bind(exchange_module.module_id)
            .fetch_one(&pool)
            .await
            .expect("module row");
    assert_eq!(
        latest,
        Some(EXCHANGE_CONTRACT),
        "module links its sale contract"
    );

    // The denormalized sort price follows the link (the
    // modules_copy_latest_contract_price trigger), tracks later
    // unified-price updates (the bid propagation trigger), and clears
    // with the ON DELETE SET NULL when the contract row goes away.
    let sort_price = |pool: sqlx::PgPool, module_id: i64| async move {
        sqlx::query_scalar::<_, Option<f64>>(
            "select latest_contract_price from modules where id = $1",
        )
        .bind(module_id)
        .fetch_one(&pool)
        .await
        .expect("module row")
    };
    assert_eq!(
        sort_price(pool.clone(), exchange_module.module_id).await,
        Some(EXCHANGE_PRICE + PLEX_AVERAGE * PLEX_QUANTITY as f64),
        "the sort price is denormalized on link",
    );
    sqlx::query("update contracts set unified_price = unified_price + 1000.0 where id = $1")
        .bind(EXCHANGE_CONTRACT)
        .execute(&pool)
        .await
        .expect("price bump");
    assert_eq!(
        sort_price(pool.clone(), exchange_module.module_id).await,
        Some(EXCHANGE_PRICE + PLEX_AVERAGE * PLEX_QUANTITY as f64 + 1000.0),
        "a unified-price change propagates to the sort price",
    );
    sqlx::query("update contracts set unified_price = unified_price - 1000.0 where id = $1")
        .bind(EXCHANGE_CONTRACT)
        .execute(&pool)
        .await
        .expect("price restore");

    let item_count: i64 =
        sqlx::query_scalar("select count(*) from contract_items where contract_id = $1")
            .bind(EXCHANGE_CONTRACT)
            .fetch_one(&pool)
            .await
            .expect("contract items");
    assert_eq!(
        item_count, 1,
        "only the abyssal module is stored as an item"
    );

    // The issuing character gains an ownership row (the legacy
    // after_public_contract_item trigger): character pages list sales
    // through public_module_ownerships.
    let ownership: Option<(i64, Option<i64>)> = sqlx::query_as(
        "select character_id, contract_id from public_module_ownerships where module_id = $1",
    )
    .bind(exchange_module.module_id)
    .fetch_optional(&pool)
    .await
    .expect("ownership row");
    let (owner, ownership_contract) = ownership.expect("the sale records an ownership");
    assert_eq!(ownership_contract, Some(EXCHANGE_CONTRACT));
    let issuer: i64 = sqlx::query_scalar("select issuer_id from contracts where id = $1")
        .bind(EXCHANGE_CONTRACT)
        .fetch_one(&pool)
        .await
        .expect("issuer");
    assert_eq!(owner, issuer);

    // Auction bids: the highest bid becomes the unified price.
    let updated = sync_auction_bids(&pool, &esi).await.expect("bids");
    assert_eq!(updated, 1);
    let (highest_bid, auction_unified): (Option<f64>, Option<f64>) =
        sqlx::query_as("select highest_bid, unified_price from contracts where id = $1")
            .bind(AUCTION_CONTRACT)
            .fetch_one(&pool)
            .await
            .expect("auction contract");
    assert_eq!(highest_bid, Some(HIGHEST_BID));
    assert_eq!(auction_unified, Some(HIGHEST_BID));

    // The module JSON carries the contract with the legacy resource keys.
    let app = mutamarket::server::test_router().await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/modules/{}", exchange_module.module_id))
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    let contract = &body["data"]["contract"];
    let mut keys: Vec<&str> = contract
        .as_object()
        .expect("contract object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "abyssal_modules_count",
            "asking_for_items",
            "date_expired",
            "date_issued",
            "id",
            "issuer",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "type",
        ],
        "contract key set diverges from the legacy resource",
    );
    assert_eq!(contract["id"], json!(EXCHANGE_CONTRACT));
    assert_eq!(contract["type"], json!("item_exchange"));
    assert_eq!(
        contract["price"],
        json!(EXCHANGE_PRICE + PLEX_AVERAGE * PLEX_QUANTITY as f64),
    );
    assert_eq!(contract["issuer"]["id"], json!(ISSUER));

    // Crash recovery: a crash between the contract upsert and the item
    // fetch leaves a known-but-itemless contract. The next cycle must fetch
    // its items even though the contract is not new to the feed.
    sqlx::query(
        "update contracts set items_synced_at = null, abyssal_modules_count = 0 where id = $1",
    )
    .bind(EXCHANGE_CONTRACT)
    .execute(&pool)
    .await
    .expect("simulate pre-item-sync state");
    sqlx::query("delete from contract_items where contract_id = $1")
        .bind(EXCHANGE_CONTRACT)
        .execute(&pool)
        .await
        .expect("drop items");
    sqlx::query("delete from modules where id = $1")
        .bind(exchange_module.module_id)
        .execute(&pool)
        .await
        .expect("drop module");

    // First retry: the item feed works but the module's dynamic data
    // fetch fails. The contract must stay pending — not be marked synced
    // with its module silently swallowed.
    fail_dynamic.store(true, Ordering::SeqCst);
    let stats = sync_region(
        &pool,
        &reference,
        &esi,
        &estimator_stub(),
        FORGE_REGION_ID,
        &|_| {},
    )
    .await
    .expect("failing retry sync");
    assert_eq!(stats.new, 0, "the crashed contract is already known");
    let still_pending: bool =
        sqlx::query_scalar("select items_synced_at is null from contracts where id = $1")
            .bind(EXCHANGE_CONTRACT)
            .fetch_one(&pool)
            .await
            .expect("pending state");
    assert!(
        still_pending,
        "a failed module import keeps the contract pending"
    );

    // Second retry with ESI healthy again: the module import lands and the
    // contract finally counts as synced.
    fail_dynamic.store(false, Ordering::SeqCst);
    sync_region(
        &pool,
        &reference,
        &esi,
        &estimator_stub(),
        FORGE_REGION_ID,
        &|_| {},
    )
    .await
    .expect("recovery sync");

    let (recovered_items, recovered_synced): (i64, bool) = sqlx::query_as(
        "select (select count(*) from contract_items where contract_id = c.id),
                c.items_synced_at is not null
         from contracts c where c.id = $1",
    )
    .bind(EXCHANGE_CONTRACT)
    .fetch_one(&pool)
    .await
    .expect("recovered contract");
    assert_eq!(recovered_items, 1, "the item sync is retried after a crash");
    assert!(recovered_synced, "the retried contract is marked synced");
    let relinked: Option<Option<i64>> =
        sqlx::query_scalar("select latest_contract_id from modules where id = $1")
            .bind(exchange_module.module_id)
            .fetch_optional(&pool)
            .await
            .expect("module query");
    assert_eq!(
        relinked,
        Some(Some(EXCHANGE_CONTRACT)),
        "the module is re-imported and relinked on retry",
    );

    // Second sync: the item exchange vanished from the feed, so it is
    // invalidated and the module unlinks.
    second_pass.store(true, Ordering::SeqCst);
    let stats = sync_region(
        &pool,
        &reference,
        &esi,
        &estimator_stub(),
        FORGE_REGION_ID,
        &|_| {},
    )
    .await
    .expect("second sync");
    assert_eq!((stats.new, stats.invalidated), (0, 1));

    let gone: Option<i64> = sqlx::query_scalar("select id from contracts where id = $1")
        .bind(EXCHANGE_CONTRACT)
        .fetch_optional(&pool)
        .await
        .expect("contract lookup");
    assert!(gone.is_none(), "invalidated contracts are removed");

    // The vanished contract is archived. With two non-abyssal items it
    // does not qualify for training data, so no status probe runs and
    // it reads unknown (the legacy qualifiesForTrainingData gate).
    let archived: Option<(String, Option<f64>, i32)> = sqlx::query_as(
        "select status, unified_price, abyssal_modules_count
         from historic_contracts where id = $1",
    )
    .bind(EXCHANGE_CONTRACT)
    .fetch_optional(&pool)
    .await
    .expect("historic lookup");
    let (status, unified_price, abyssal_count) = archived.expect("contract archived");
    assert_eq!(status, "unknown");
    assert_eq!(abyssal_count, 1);
    assert!(unified_price.is_some(), "the unified price is carried over");
    let archived_item: Option<i64> = sqlx::query_scalar(
        "select item_id from historic_contract_items
         where historic_contract_id = $1 and item_id = $2",
    )
    .bind(EXCHANGE_CONTRACT)
    .bind(exchange_module.module_id)
    .fetch_optional(&pool)
    .await
    .expect("historic item lookup");
    assert_eq!(
        archived_item,
        Some(exchange_module.module_id),
        "the module item is copied"
    );

    // Third sync: the auction vanishes too. It qualifies for training
    // data (one abyssal module, nothing else), so the status probe runs
    // and reads the accepted-by-player answer.
    third_pass.store(true, Ordering::SeqCst);
    sync_region(
        &pool,
        &reference,
        &esi,
        &estimator_stub(),
        FORGE_REGION_ID,
        &|_| {},
    )
    .await
    .expect("third sync");
    let auction_status: Option<String> =
        sqlx::query_scalar("select status from historic_contracts where id = $1")
            .bind(AUCTION_CONTRACT)
            .fetch_optional(&pool)
            .await
            .expect("auction historic lookup");
    assert_eq!(
        auction_status.as_deref(),
        Some("completed"),
        "the probe reads the accepted contract",
    );

    let unlinked: Option<i64> =
        sqlx::query_scalar("select latest_contract_id from modules where id = $1")
            .bind(exchange_module.module_id)
            .fetch_one(&pool)
            .await
            .expect("module row");
    assert!(
        unlinked.is_none(),
        "the module unlinks when its contract dies"
    );
    let stale_price: Option<f64> =
        sqlx::query_scalar("select latest_contract_price from modules where id = $1")
            .bind(exchange_module.module_id)
            .fetch_one(&pool)
            .await
            .expect("module row");
    assert!(
        stale_price.is_none(),
        "the denormalized sort price clears with the link"
    );

    let imports: i64 =
        sqlx::query_scalar("select count(*) from contract_imports where region_id = $1")
            .bind(FORGE_REGION_ID)
            .fetch_one(&pool)
            .await
            .expect("imports");
    assert!(imports >= 2, "every run books a contract import row");
}
