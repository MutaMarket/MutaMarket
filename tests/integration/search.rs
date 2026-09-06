//! Behavior tests for the module search query, ported from the legacy
//! QueryService: type scoping, sorting, attribute/meta/bar filters, and
//! their legacy error semantics — through the API and the browser page.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use tower::ServiceExt;

async fn get(app: &Router, path: &str) -> (StatusCode, serde_json::Value, String) {
    get_as(app, path, None).await
}

/// Like [`get`] with the session cookie of a signed-in user.
async fn get_as(
    app: &Router,
    path: &str,
    session: Option<&str>,
) -> (StatusCode, serde_json::Value, String) {
    let mut builder = Request::builder().uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, json, text)
}

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
/// The signed-in pilot of the personal-modules assertions.
const SEARCH_PILOT_CHARACTER_ID: i64 = 90999997;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

/// The default listing order, `modules.id desc`.
fn sorted_desc(ids: &[i64]) -> Vec<i64> {
    let mut ids = ids.to_vec();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    ids
}

/// The ids of a bare card array (the browser endpoints).
fn card_ids(body: &serde_json::Value) -> Vec<i64> {
    body.as_array()
        .expect("bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect()
}

fn data_ids(body: &serde_json::Value) -> Vec<i64> {
    body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect()
}

async fn ingest(
    pool: &sqlx::PgPool,
    reference: &ReferenceData,
    type_id: i64,
    module: &common::ModuleFixture,
) {
    process_module(
        pool,
        reference,
        &estimator_stub(),
        type_id,
        module.module_id,
        &DogmaItem {
            created_by: module.creator_id,
            source_type_id: module.source_type_id,
            mutator_type_id: module.mutaplasmid_id,
            dogma_attributes: common::fixture_dogma(module),
        },
    )
    .await
    .expect("process module");
}

#[tokio::test]
async fn search_filters_and_sorts_like_the_legacy_query_service() {
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

    let fixtures = common::load_module_fixtures();
    let mwd = fixtures
        .iter()
        .find(|f| f.type_id == 47408)
        .expect("MWD fixture");
    let web = fixtures
        .iter()
        .find(|f| f.type_id == 47702)
        .expect("web fixture");
    let bcs = fixtures
        .iter()
        .find(|f| f.type_id == 49726)
        .expect("BCS fixture");

    // Two 50MN MWDs (worst and best roll), one web, and the best-rolled
    // Ballistic Control System, which carries a gold bar.
    let mwd_worst = &mwd.modules[0];
    let mwd_best = mwd.modules.last().expect("modules");
    let web_module = &web.modules[0];
    let gold_module = bcs.modules.last().expect("modules");
    assert!(
        gold_module
            .expected
            .attributes
            .iter()
            .any(|attribute| attribute.bar == 1),
        "fixture expectation: the BCS module has a gold bar",
    );

    // An extra module that stays unlisted (no contract): visible on the
    // all-modules page only.
    let mwd_unlisted = &mwd.modules[1];
    // A module that becomes a MutaMarket sell listing (a public asset, no
    // contract) halfway through: the contract filters must keep it.
    let mwd_public = &mwd.modules[2];

    for (type_id, module) in [
        (mwd.type_id, mwd_worst),
        (mwd.type_id, mwd_best),
        (mwd.type_id, mwd_unlisted),
        (mwd.type_id, mwd_public),
        (web.type_id, web_module),
        (bcs.type_id, gold_module),
    ] {
        ingest(&pool, &reference, type_id, module).await;
    }

    // The estimate-filter assertions rely on a clean slate; other suites
    // (the legacy importer test) may have left estimates on these
    // fixture modules.
    sqlx::query("update modules set estimated_value = null where id = any($1)")
        .bind(vec![
            mwd_worst.module_id,
            mwd_best.module_id,
            mwd_unlisted.module_id,
            mwd_public.module_id,
            web_module.module_id,
            gold_module.module_id,
        ])
        .execute(&pool)
        .await
        .expect("clear estimates");

    // For-sale state mirroring the legacy browse visibility, with the
    // spread needed by the price and contract filters.
    common::attach_contract(
        &pool,
        mwd_worst.module_id,
        800_001,
        "item_exchange",
        100_000_000.0,
        1,
        0,
        0,
    )
    .await;
    common::attach_contract(
        &pool,
        mwd_best.module_id,
        800_002,
        "auction",
        500_000_000.0,
        1,
        2,
        0,
    )
    .await;
    common::attach_contract(
        &pool,
        web_module.module_id,
        800_003,
        "item_exchange",
        200_000_000.0,
        1,
        1,
        500,
    )
    .await;
    common::attach_contract(
        &pool,
        gold_module.module_id,
        800_004,
        "item_exchange",
        900_000_000.0,
        1,
        0,
        0,
    )
    .await;
    sqlx::query("update modules set latest_contract_id = null where id = any($1)")
        .bind(vec![mwd_unlisted.module_id, mwd_public.module_id])
        .execute(&pool)
        .await
        .expect("unlist modules");
    // Other suites may have published the soon-to-be public module; it
    // starts unlisted here and is published below.
    sqlx::query("delete from public_assets where module_id = $1")
        .bind(mwd_public.module_id)
        .execute(&pool)
        .await
        .expect("unpublish prior assets");

    let app = mutamarket::server::test_router().await;

    // Type scoping: only modules of the type, by id and by slug.
    let (status, body, _) = get(&app, "/api/modules/type/47408").await;
    assert_eq!(status, StatusCode::OK);
    let ids = data_ids(&body);
    assert!(ids.contains(&mwd_worst.module_id) && ids.contains(&mwd_best.module_id));
    assert!(
        !ids.contains(&web_module.module_id),
        "other types are excluded"
    );
    assert!(!ids.contains(&gold_module.module_id));

    let (_, by_slug, _) = get(&app, "/api/modules/type/50mn-abyssal-microwarpdrive").await;
    assert_eq!(data_ids(&by_slug), ids, "slug resolves to the same type");

    // Sorting by roll quality, both directions.
    let (_, ascending, _) = get(&app, "/api/modules/type/47408/sort/fraction/asc").await;
    assert_eq!(
        data_ids(&ascending),
        vec![mwd_worst.module_id, mwd_best.module_id]
    );
    let (_, descending, _) = get(&app, "/api/modules/type/47408/sort/fraction/desc").await;
    assert_eq!(
        data_ids(&descending),
        vec![mwd_best.module_id, mwd_worst.module_id]
    );

    // Sorting by a rolled attribute, addressed by attribute name.
    let sort_attribute = &mwd_worst.expected.attributes[0];
    let attribute_name: String = sqlx::query_scalar("select name from attributes where id = $1")
        .bind(sort_attribute.attribute_id)
        .fetch_one(&pool)
        .await
        .expect("attribute name");
    let best_value = mwd_best
        .expected
        .attributes
        .iter()
        .find(|attribute| attribute.attribute_id == sort_attribute.attribute_id)
        .expect("attribute on both modules")
        .value;
    let expected_order = if sort_attribute.value < best_value {
        vec![mwd_worst.module_id, mwd_best.module_id]
    } else {
        vec![mwd_best.module_id, mwd_worst.module_id]
    };
    let (status, by_attribute, _) = get(
        &app,
        &format!("/api/modules/type/47408/sort/{attribute_name}/asc"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data_ids(&by_attribute), expected_order);

    // Attribute range filter: bounds that only match the worst roll.
    let low = sort_attribute.value.min(best_value);
    let range = format!("{}-{}", low - 1.0, low + 1.0);
    let (status, filtered, _) = get(
        &app,
        &format!("/api/modules/type/47408/attributes/{attribute_name}/{range}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let expected_id = if sort_attribute.value < best_value {
        mwd_worst.module_id
    } else {
        mwd_best.module_id
    };
    assert_eq!(data_ids(&filtered), vec![expected_id]);

    // The legacy query builder lowercases attribute names in URLs; the
    // filter must resolve them case-insensitively.
    let (status, filtered_lower, _) = get(
        &app,
        &format!(
            "/api/modules/type/47408/attributes/{}/{range}",
            attribute_name.to_lowercase(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data_ids(&filtered_lower), vec![expected_id]);

    // Cursor pagination: the legacy simple cursor contract — data plus
    // links {first,last,prev,next} and meta {path,per_page,next_cursor,
    // prev_cursor}; first/last are always null.
    let (status, page, _) = get(&app, "/api/modules/type/47408").await;
    assert_eq!(status, StatusCode::OK);
    let mut keys: Vec<&str> = page
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["data", "links", "meta"]);
    let mut link_keys: Vec<&str> = page["links"]
        .as_object()
        .expect("links")
        .keys()
        .map(String::as_str)
        .collect();
    link_keys.sort_unstable();
    assert_eq!(link_keys, ["first", "last", "next", "prev"]);
    let mut meta_keys: Vec<&str> = page["meta"]
        .as_object()
        .expect("meta")
        .keys()
        .map(String::as_str)
        .collect();
    meta_keys.sort_unstable();
    assert_eq!(
        meta_keys,
        ["next_cursor", "path", "per_page", "prev_cursor"]
    );
    assert_eq!(page["links"]["first"], serde_json::Value::Null);
    assert_eq!(page["links"]["last"], serde_json::Value::Null);
    assert_eq!(page["meta"]["per_page"], serde_json::json!(100));
    assert_eq!(
        page["meta"]["path"],
        serde_json::json!("/api/modules/type/47408")
    );
    // Both fixture modules fit on one page: no cursors.
    assert_eq!(page["meta"]["next_cursor"], serde_json::Value::Null);
    assert_eq!(page["meta"]["prev_cursor"], serde_json::Value::Null);

    // Gold bar flag: only the BCS module across its type.
    let (_, gold, _) = get(&app, "/api/modules/type/49726/goldbar").await;
    assert_eq!(data_ids(&gold), vec![gold_module.module_id]);
    let (_, brown, _) = get(&app, "/api/modules/type/49726/brownbar").await;
    assert!(
        !data_ids(&brown).contains(&gold_module.module_id)
            || gold_module.expected.attributes.iter().any(|a| a.bar == -1),
        "brownbar only matches modules with a brown bar",
    );

    // Meta group filter: the MWD's source meta group matches, others do not.
    let source_meta_group: Option<i64> =
        sqlx::query_scalar("select meta_group_id from types where id = $1")
            .bind(mwd_worst.source_type_id)
            .fetch_one(&pool)
            .await
            .expect("source meta group");
    let source_meta_group = source_meta_group.expect("fixture source has a meta group");
    let (_, same_group, _) = get(
        &app,
        &format!("/api/modules/type/47408/meta-group/{source_meta_group}"),
    )
    .await;
    assert!(data_ids(&same_group).contains(&mwd_worst.module_id));
    let unused_group = if source_meta_group == 5 { 6 } else { 5 };
    let (_, other_group, _) = get(
        &app,
        &format!("/api/modules/type/47408/meta-group/{unused_group}"),
    )
    .await;
    assert!(data_ids(&other_group).is_empty());

    // Estimated value bounds exclude modules without an estimate; a zero
    // lower bound disables the filter like the legacy PHP truthiness.
    let (_, valued, _) = get(&app, "/api/modules/type/47408/estimated-value/1000").await;
    assert!(
        data_ids(&valued).is_empty(),
        "no estimates yet, so no matches"
    );
    let (_, zero_bound, _) = get(&app, "/api/modules/type/47408/estimated-value/0-5000").await;
    assert_eq!(
        data_ids(&zero_bound).len(),
        2,
        "zero lower bound disables the filter"
    );

    // Unlisted modules are hidden from the browse pages but shown on the
    // all-modules page, like the legacy visibility split.
    let (_, listed_only, _) = get(&app, "/api/modules/type/47408").await;
    assert!(!data_ids(&listed_only).contains(&mwd_unlisted.module_id));
    let (status, all_cards, _) = get(
        &app,
        "/api/module-cards/type/50mn-abyssal-microwarpdrive?unlisted=true",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        all_cards
            .as_array()
            .expect("bare card array")
            .iter()
            .any(|module| module["id"] == serde_json::json!(mwd_unlisted.module_id)),
        "the all-modules set includes the unlisted module",
    );

    // From here on a third MWD is for sale as a MutaMarket sell listing
    // (a public asset, no contract). The legacy index OR-ed the public
    // listings onto the contract branch, so every contract-only filter
    // below keeps it; `contracts-only` is what drops it.
    common::publish_asset(&pool, mwd_public.module_id, mwd.type_id).await;
    let (_, with_public, _) = get(&app, "/api/modules/type/47408").await;
    assert_eq!(
        data_ids(&with_public),
        sorted_desc(&[
            mwd_worst.module_id,
            mwd_best.module_id,
            mwd_public.module_id
        ]),
        "the public listing joins the for-sale set, newest module first",
    );

    // Price sorting over the unified contract price, both directions. A
    // public listing has no contract price and sorts as the lowest, like
    // the legacy coalesce to zero.
    let (_, price_asc, _) = get(&app, "/api/modules/type/47408/sort/price/asc").await;
    assert_eq!(
        data_ids(&price_asc),
        vec![
            mwd_public.module_id,
            mwd_worst.module_id,
            mwd_best.module_id
        ]
    );
    let (_, price_desc, _) = get(&app, "/api/modules/type/47408/sort/price/desc").await;
    assert_eq!(
        data_ids(&price_desc),
        vec![
            mwd_best.module_id,
            mwd_worst.module_id,
            mwd_public.module_id
        ]
    );

    // Contract price bounds: a single number is a maximum, a range is
    // inclusive, and a zero lower bound disables the filter. The public
    // listing passes every bound.
    let (_, max_bound, _) = get(&app, "/api/modules/type/47408/contract-price/300000000").await;
    assert_eq!(
        data_ids(&max_bound),
        sorted_desc(&[mwd_worst.module_id, mwd_public.module_id])
    );
    let (_, range_bound, _) = get(
        &app,
        "/api/modules/type/47408/contract-price/50000000-600000000",
    )
    .await;
    assert_eq!(data_ids(&range_bound).len(), 3);
    let (_, zero_bound_price, _) = get(&app, "/api/modules/type/47408/contract-price/0-100").await;
    assert_eq!(
        data_ids(&zero_bound_price).len(),
        3,
        "zero lower bound disables the filter"
    );

    // Contract type flags narrow the contract branch only.
    let (_, auctions, _) = get(&app, "/api/modules/type/47408/auction").await;
    assert_eq!(
        data_ids(&auctions),
        sorted_desc(&[mwd_best.module_id, mwd_public.module_id])
    );
    let (_, exchanges, _) = get(&app, "/api/modules/type/47408/item-exchange").await;
    assert_eq!(
        data_ids(&exchanges),
        sorted_desc(&[mwd_worst.module_id, mwd_public.module_id])
    );

    // Single-item and without-other-items rules, likewise.
    let (_, single, _) = get(&app, "/api/modules/type/47408/no-multi-item-contracts").await;
    assert_eq!(
        data_ids(&single),
        sorted_desc(&[mwd_worst.module_id, mwd_public.module_id])
    );
    let (_, clean, _) = get(&app, "/api/modules/type/47408/without-other-items").await;
    assert_eq!(
        data_ids(&clean),
        sorted_desc(&[mwd_worst.module_id, mwd_public.module_id])
    );

    // `contracts-only` removes the public branch: the contract filters
    // then stand alone.
    let (_, auctions_only, _) = get(&app, "/api/modules/type/47408/auction/contracts-only").await;
    assert_eq!(data_ids(&auctions_only), vec![mwd_best.module_id]);
    let (_, exchanges_only, _) =
        get(&app, "/api/modules/type/47408/item-exchange/contracts-only").await;
    assert_eq!(data_ids(&exchanges_only), vec![mwd_worst.module_id]);
    let (_, priced_only, _) = get(
        &app,
        "/api/modules/type/47408/contract-price/300000000/contracts-only",
    )
    .await;
    assert_eq!(data_ids(&priced_only), vec![mwd_worst.module_id]);
    let (_, plex_ok, _) = get(&app, "/api/modules/type/47702/without-other-items").await;
    assert_eq!(
        data_ids(&plex_ok),
        vec![web_module.module_id],
        "one extra item is fine when it is asked-for PLEX",
    );

    // Jita 4-4 pins the current contract's start station (the legacy
    // inJita). It is a common filter, so the contract-less listing drops.
    sqlx::query("update contracts set start_location_id = $2 where id = $1")
        .bind(800_001)
        .bind(60003760i64)
        .execute(&pool)
        .await
        .expect("move contract to Jita");
    let (status, jita, _) = get(&app, "/api/modules/type/47408/in-jita").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data_ids(&jita), vec![mwd_worst.module_id]);

    // Free-text search over the mutaplasmid, type and source type names,
    // case-insensitively like MySQL's LIKE.
    let (status, found, _) = get(&app, "/api/modules/type/47408/search/MICROWARP").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data_ids(&found).len(), 3, "the type name matches");
    let (_, missed, _) = get(&app, "/api/modules/type/47408/search/no-such-module-name").await;
    assert!(data_ids(&missed).is_empty());
    let (_, bare, _) = get(&app, "/api/modules/type/47408/search").await;
    assert_eq!(
        data_ids(&bare).len(),
        3,
        "a bare search segment filters nothing"
    );

    // Date sorts: the current contract's ESI issue date and its import
    // date, which need not agree. The contract-less listing sorts as the
    // oldest either way (the legacy coalesce to the epoch).
    sqlx::query(
        "update contracts set date_issued = now() - interval '2 days',
                              created_at = now() - interval '1 hour'
         where id = 800001",
    )
    .execute(&pool)
    .await
    .expect("date the worst roll's contract");
    sqlx::query(
        "update contracts set date_issued = now() - interval '1 day',
                              created_at = now() - interval '2 hours'
         where id = 800002",
    )
    .execute(&pool)
    .await
    .expect("date the best roll's contract");
    let (status, issued_asc, _) = get(&app, "/api/modules/type/47408/sort/contract-date/asc").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        data_ids(&issued_asc),
        vec![
            mwd_public.module_id,
            mwd_worst.module_id,
            mwd_best.module_id
        ]
    );
    let (_, issued_desc, _) = get(&app, "/api/modules/type/47408/sort/contract-date/desc").await;
    assert_eq!(
        data_ids(&issued_desc),
        vec![
            mwd_best.module_id,
            mwd_worst.module_id,
            mwd_public.module_id
        ]
    );

    // The date-added sort reads the public listing rows (contract or
    // published asset), so the direct listing dates too: published half
    // an hour ago it is the newest, the best roll's contract (two hours)
    // the oldest. A module without a listing row sorts as the oldest.
    sqlx::query("delete from public_module_ownerships where module_id = any($1)")
        .bind(vec![
            mwd_public.module_id,
            mwd_best.module_id,
            mwd_worst.module_id,
        ])
        .execute(&pool)
        .await
        .expect("clean listing rows");
    for (module_id, contract_id, age) in [
        (mwd_worst.module_id, 800001_i64, "1 hour"),
        (mwd_best.module_id, 800002_i64, "2 hours"),
    ] {
        sqlx::query(
            "insert into public_module_ownerships (character_id, module_id, contract_id, updated_at)
             values (90999999, $1, $2, now() - $3::interval)",
        )
        .bind(module_id)
        .bind(contract_id)
        .bind(age)
        .execute(&pool)
        .await
        .expect("contract listing row");
    }
    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id, public_asset_id, updated_at)
         select pa.character_id, pa.module_id, pa.id, now() - interval '30 minutes'
         from public_assets pa where pa.module_id = $1",
    )
    .bind(mwd_public.module_id)
    .execute(&pool)
    .await
    .expect("asset listing row");
    let (status, added_asc, _) = get(&app, "/api/modules/type/47408/sort/date-added/asc").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        data_ids(&added_asc),
        vec![
            mwd_best.module_id,
            mwd_worst.module_id,
            mwd_public.module_id
        ]
    );
    let (_, added_desc, _) = get(&app, "/api/modules/type/47408/sort/date-added/desc").await;
    assert_eq!(
        data_ids(&added_desc),
        vec![
            mwd_public.module_id,
            mwd_worst.module_id,
            mwd_best.module_id
        ]
    );
    sqlx::query("delete from public_module_ownerships where module_id = any($1)")
        .bind(vec![
            mwd_public.module_id,
            mwd_best.module_id,
            mwd_worst.module_id,
        ])
        .execute(&pool)
        .await
        .expect("clean listing rows");

    // The API's region_id sits outside the where-group like the legacy
    // whereHas, so it drops the public listing as well.
    let (status, forge, _) = get(&app, "/api/modules/type/47408?region_id=10000002").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        data_ids(&forge),
        sorted_desc(&[mwd_worst.module_id, mwd_best.module_id])
    );
    let (_, elsewhere, _) = get(&app, "/api/modules/type/47408?region_id=10000043").await;
    assert!(data_ids(&elsewhere).is_empty());

    // with-personal-modules ORs the signed-in account's own asset modules
    // into the market set (the unlisted MWD sits in this pilot's hangar);
    // guests see the market set unchanged.
    sqlx::query("delete from assets where character_id = $1")
        .bind(SEARCH_PILOT_CHARACTER_ID)
        .execute(&pool)
        .await
        .expect("clean pilot assets");
    sqlx::query("delete from characters where id = $1")
        .bind(SEARCH_PILOT_CHARACTER_ID)
        .execute(&pool)
        .await
        .expect("clean pilot");
    let pilot_user: i64 =
        sqlx::query_scalar("insert into users (name) values ('Search Pilot') returning id")
            .fetch_one(&pool)
            .await
            .expect("create user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Search Pilot', $2)")
        .bind(SEARCH_PILOT_CHARACTER_ID)
        .bind(pilot_user)
        .execute(&pool)
        .await
        .expect("create character");
    sqlx::query(
        "insert into assets
         (character_id, item_id, type_id, location_flag, location_type, quantity, is_abyssal)
         values ($1, $2, $3, 'Hangar', 'station', 1, true)",
    )
    .bind(SEARCH_PILOT_CHARACTER_ID)
    .bind(mwd_unlisted.module_id)
    .bind(mwd.type_id)
    .execute(&pool)
    .await
    .expect("seed the pilot's module");
    let session = mutamarket::auth::session::create_session(
        &pool,
        pilot_user,
        Some(SEARCH_PILOT_CHARACTER_ID),
    )
    .await
    .expect("create session");
    let personal_path = "/api/module-cards/type/47408/with-personal-modules";
    let (status, as_guest, _) = get(&app, personal_path).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!card_ids(&as_guest).contains(&mwd_unlisted.module_id));
    let (status, as_pilot, _) = get_as(&app, personal_path, Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(card_ids(&as_pilot).contains(&mwd_unlisted.module_id));
    assert!(card_ids(&as_pilot).contains(&mwd_worst.module_id));
    let (_, market_only, _) = get_as(&app, "/api/module-cards/type/47408", Some(&session)).await;
    assert!(
        !card_ids(&market_only).contains(&mwd_unlisted.module_id),
        "without the option the market set stays the market set",
    );

    // Legacy error semantics.
    let (status, body, _) = get(&app, "/api/modules/type/not-a-real-type-anywhere").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        serde_json::json!("Please provide a valid type.")
    );

    let (status, body, _) = get(&app, "/api/modules/type/47408/meta-group/imaginary").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        serde_json::json!("You provided an invalid meta group: imaginary"),
    );

    let (status, body, _) = get(&app, "/api/modules/type/47408/attributes/notanattribute/5").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        serde_json::json!("Unknown attribute: notanattribute")
    );

    let (status, body, _) = get(&app, "/api/modules/sort/50/asc/goldbar").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        serde_json::json!("Module type must be specified when sorting by attribute."),
    );

    // The browser card endpoint runs the same search but returns the bare
    // card array (page size 30): same modules and order as the legacy-API
    // index for the same query, identical card serialization.
    let (status, cards, _) = get(&app, "/api/module-cards/type/47408").await;
    assert_eq!(status, StatusCode::OK);
    let (_, index, _) = get(&app, "/api/modules/type/47408").await;
    let card_ids: Vec<i64> = cards
        .as_array()
        .expect("bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect();
    assert_eq!(card_ids, data_ids(&index));
    crate::common::assert_default_module_keys(&cards[0], false, &[]);
    assert_eq!(
        cards[0], index["data"][0],
        "cards serialize like the index resource"
    );

    // The page/N option offsets the card set like the legacy paginator:
    // both matching modules fit on page one, so page two is empty.
    let (status, second_page, _) = get(&app, "/api/module-cards/type/47408/page/2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second_page.as_array().expect("bare card array").len(), 0);

    // unlisted=true (the all-modules page) includes modules without a
    // contract; the default browser set does not.
    let (_, all_cards, _) = get(&app, "/api/module-cards/type/47408?unlisted=true").await;
    let all_ids: Vec<i64> = all_cards
        .as_array()
        .expect("bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect();
    assert!(all_ids.contains(&mwd_unlisted.module_id));
    assert!(!card_ids.contains(&mwd_unlisted.module_id));

    // The unfiltered browser home set serves for-sale modules of any type.
    let (status, home, _) = get(&app, "/api/module-cards").await;
    assert_eq!(status, StatusCode::OK);
    let home_ids: Vec<i64> = home
        .as_array()
        .expect("bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect();
    assert!(home_ids.contains(&mwd_worst.module_id));
    assert!(home_ids.contains(&web_module.module_id));

    // Card search failures carry the legacy statuses and messages.
    let (status, body, _) = get(&app, "/api/module-cards/type/not-a-real-type-anywhere").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        serde_json::json!("Please provide a valid type.")
    );
    let (status, body, _) = get(
        &app,
        "/api/module-cards/type/47408/attributes/notanattribute/5",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        serde_json::json!("Unknown attribute: notanattribute")
    );

    // The market stats strip payload.
    let (status, stats, _) = get(&app, "/api/module-stats").await;
    assert_eq!(status, StatusCode::OK);
    let mut stats_keys: Vec<&str> = stats
        .as_object()
        .expect("stats object")
        .keys()
        .map(String::as_str)
        .collect();
    stats_keys.sort_unstable();
    assert_eq!(
        stats_keys,
        [
            "added_last_day_count",
            "added_last_hour_count",
            "added_last_week_count",
            "auctions_count",
            "brownbars_count",
            "contracts_count",
            "diamondbars_count",
            "goldbars_count",
            "item_exchanges_count",
            "listed_count",
            "total_count",
        ],
    );
    assert!(
        stats["total_count"].as_i64().expect("count") >= 4,
        "the seeded modules count"
    );

    // Bar totals: the default counts only for-sale modules, the
    // all-modules variant (`unlisted=true`) spans the whole archive. A
    // diamond roll on a contract-less module shows up in the second
    // count only.
    let (unlisted_module, original_bar): (i64, i16) = sqlx::query_as(
        "select ma.module_id, ma.bar from mutated_attributes ma
         join modules m on m.id = ma.module_id
         where m.latest_contract_id is null order by ma.id limit 1",
    )
    .fetch_one(&pool)
    .await
    .expect("a contract-less module");
    sqlx::query("update mutated_attributes set bar = 2 where module_id = $1")
        .bind(unlisted_module)
        .execute(&pool)
        .await
        .expect("stamp diamond");
    let (_, listed, _) = get(&app, "/api/module-stats").await;
    let (status, unlisted, _) = get(&app, "/api/module-stats?unlisted=true").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        unlisted["diamondbars_count"].as_i64().expect("count")
            > listed["diamondbars_count"].as_i64().expect("count"),
        "the archive-wide count sees the contract-less diamond: {} vs {}",
        unlisted["diamondbars_count"],
        listed["diamondbars_count"],
    );
    sqlx::query("update mutated_attributes set bar = $2 where module_id = $1")
        .bind(unlisted_module)
        .bind(original_bar)
        .execute(&pool)
        .await
        .expect("restore bar");

    // The filter panel resolves the type like the search does.
    let (status, panel, _) = get(&app, "/api/filter-panel/50mn-abyssal-microwarpdrive").await;
    assert_eq!(status, StatusCode::OK);
    let mut panel_keys: Vec<&str> = panel
        .as_object()
        .expect("panel object")
        .keys()
        .map(String::as_str)
        .collect();
    panel_keys.sort_unstable();
    assert_eq!(
        panel_keys,
        ["attributes", "source_types", "type_id", "type_name"]
    );
    assert_eq!(panel["type_id"], serde_json::json!(47408));
    assert_eq!(
        panel["type_name"],
        serde_json::json!("50MN Abyssal Microwarpdrive")
    );
    let attributes = panel["attributes"].as_array().expect("attributes");
    assert!(!attributes.is_empty());
    for attribute in attributes {
        let mut keys: Vec<&str> = attribute
            .as_object()
            .expect("attribute object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "attribute_id",
                "best",
                "display_name",
                "high_is_good",
                "is_virtual",
                "name",
                "unit_display_name",
                "unit_name",
                "worst",
            ],
        );
    }

    // The source-type rows powering the slider pips: published input
    // types with their base values for the panel's attributes, meta
    // rank then name.
    let source_types = panel["source_types"].as_array().expect("source types");
    assert!(!source_types.is_empty());
    for source_type in source_types {
        let mut keys: Vec<&str> = source_type
            .as_object()
            .expect("source type object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["attributes", "id", "meta_group_id", "meta_level", "name"]
        );
        for value in source_type["attributes"].as_array().expect("values") {
            let mut value_keys: Vec<&str> = value
                .as_object()
                .expect("value object")
                .keys()
                .map(String::as_str)
                .collect();
            value_keys.sort_unstable();
            assert_eq!(value_keys, ["attribute_id", "value"]);
        }
    }
    let ranks: Vec<i64> = source_types
        .iter()
        .map(|source_type| match source_type["meta_group_id"].as_i64() {
            Some(1) => 1,
            Some(2) => 2,
            Some(3) => 3,
            Some(4) => 4,
            Some(6) => 5,
            Some(5) => 6,
            other => other.unwrap_or(i64::MAX),
        })
        .collect();
    assert!(
        ranks.windows(2).all(|pair| pair[0] <= pair[1]),
        "meta-rank order: {ranks:?}"
    );

    let (status, body, _) = get(&app, "/api/filter-panel/not-a-real-type-anywhere").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        serde_json::json!("Please provide a valid type.")
    );

    // Type resolution by slug matches the card endpoint too.
    let (status, by_slug_cards, _) =
        get(&app, "/api/module-cards/type/50mn-abyssal-microwarpdrive").await;
    assert_eq!(status, StatusCode::OK);
    let by_slug_ids: Vec<i64> = by_slug_cards
        .as_array()
        .expect("bare card array")
        .iter()
        .filter_map(|module| module["id"].as_i64())
        .collect();
    assert_eq!(by_slug_ids, card_ids);
}
