//! Behavior tests for public contract ingestion against a mock ESI: region
//! sync with classification and module linking, unified prices including
//! asked-for PLEX, auction bids, invalidation, and the contract emitted in
//! the module JSON with the legacy resource key set.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

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

const EXCHANGE_CONTRACT: i64 = 900_001;
const AUCTION_CONTRACT: i64 = 900_002;
const ISSUER: i64 = 91_000_001;
const PLEX_QUANTITY: i64 = 500;
const PLEX_AVERAGE: f64 = 5_000_000.0;
const EXCHANGE_PRICE: f64 = 1_000_000_000.0;
const AUCTION_START_PRICE: f64 = 2_000_000_000.0;
const HIGHEST_BID: f64 = 3_000_000_000.0;

/// Mock ESI: contracts feed (second pass drops the item exchange), items,
/// bids, market history and dynamic items for the two fixture modules.
fn mock_esi(
    second_pass: Arc<AtomicBool>,
    exchange_module: serde_json::Value,
    auction_module: serde_json::Value,
) -> Router {
    let exchange_item = exchange_module["item_id"].as_i64().expect("item id");
    let auction_item = auction_module["item_id"].as_i64().expect("item id");
    let exchange_type = exchange_module["type_id"].as_i64().expect("type id");
    let auction_type = auction_module["type_id"].as_i64().expect("type id");

    Router::new()
        .route(
            "/latest/contracts/public/{region_id}/",
            get(move || {
                let second_pass = second_pass.clone();
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

                    let feed = if second_pass.load(Ordering::SeqCst) {
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
            get(move |AxumPath(contract_id): AxumPath<i64>| async move {
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
                async move {
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

#[tokio::test]
async fn contracts_sync_ingests_classifies_and_links_modules() {
    let pool = db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict (id) do nothing")
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

    // Idempotency across runs: previous runs leave the auction behind.
    sqlx::query("delete from contracts where id = any($1)")
        .bind(vec![EXCHANGE_CONTRACT, AUCTION_CONTRACT])
        .execute(&pool)
        .await
        .expect("clean contracts");

    let fixtures = common::load_module_fixtures();
    let exchange_fixture = fixtures.iter().find(|f| f.type_id == 47736).expect("fixture");
    let auction_fixture = fixtures.iter().find(|f| f.type_id == 47740).expect("fixture");
    let exchange_module = &exchange_fixture.modules[0];
    let auction_module = &auction_fixture.modules[0];

    let second_pass = Arc::new(AtomicBool::new(false));
    let esi_url = start_mock(mock_esi(
        second_pass.clone(),
        dogma_payload(exchange_fixture.type_id, exchange_module),
        dogma_payload(auction_fixture.type_id, auction_module),
    ))
    .await;
    let esi = EsiClient::new(&esi_url);

    // Market history first: the PLEX average feeds unified prices.
    let days = sync_plex_market_history(&pool, &esi).await.expect("plex history");
    assert_eq!(days, 2);
    assert_eq!(plex_average(&pool).await.expect("average"), Some(PLEX_AVERAGE));

    // First sync: the courier is filtered, both relevant contracts land,
    // items are fetched and classified, modules imported and linked.
    let stats = sync_region(&pool, &reference, &esi, FORGE_REGION_ID)
        .await
        .expect("sync region");
    assert_eq!((stats.total, stats.relevant, stats.new, stats.invalidated), (3, 2, 2, 0));

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
    assert_eq!(latest, Some(EXCHANGE_CONTRACT), "module links its sale contract");

    let item_count: i64 =
        sqlx::query_scalar("select count(*) from contract_items where contract_id = $1")
            .bind(EXCHANGE_CONTRACT)
            .fetch_one(&pool)
            .await
            .expect("contract items");
    assert_eq!(item_count, 1, "only the abyssal module is stored as an item");

    // Auction bids: the highest bid becomes the unified price.
    let updated = sync_auction_bids(&pool, &esi).await.expect("bids");
    assert_eq!(updated, 1);
    let (highest_bid, auction_unified): (Option<f64>, Option<f64>) = sqlx::query_as(
        "select highest_bid, unified_price from contracts where id = $1",
    )
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
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
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

    // Second sync: the item exchange vanished from the feed, so it is
    // invalidated and the module unlinks.
    second_pass.store(true, Ordering::SeqCst);
    let stats = sync_region(&pool, &reference, &esi, FORGE_REGION_ID)
        .await
        .expect("second sync");
    assert_eq!((stats.new, stats.invalidated), (0, 1));

    let gone: Option<i64> = sqlx::query_scalar("select id from contracts where id = $1")
        .bind(EXCHANGE_CONTRACT)
        .fetch_optional(&pool)
        .await
        .expect("contract lookup");
    assert!(gone.is_none(), "invalidated contracts are removed");

    let unlinked: Option<i64> =
        sqlx::query_scalar("select latest_contract_id from modules where id = $1")
            .bind(exchange_module.module_id)
            .fetch_one(&pool)
            .await
            .expect("module row");
    assert!(unlinked.is_none(), "the module unlinks when its contract dies");

    let imports: i64 = sqlx::query_scalar(
        "select count(*) from contract_imports where region_id = $1",
    )
    .bind(FORGE_REGION_ID)
    .fetch_one(&pool)
    .await
    .expect("imports");
    assert!(imports >= 2, "every run books a contract import row");
}
