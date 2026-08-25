//! Behavior tests for the module search query, ported from the legacy
//! QueryService: type scoping, sorting, attribute/meta/bar filters, and
//! their legacy error semantics — through the API and the browser page.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use tower::ServiceExt;

async fn get(app: &Router, path: &str) -> (StatusCode, serde_json::Value, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, json, text)
}


/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::EstimatorClient {
    mutamarket::estimator::EstimatorClient::new("http://127.0.0.1:9")
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
    seed_reference(&pool, &tables).await.expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    let fixtures = common::load_module_fixtures();
    let mwd = fixtures.iter().find(|f| f.type_id == 47408).expect("MWD fixture");
    let web = fixtures.iter().find(|f| f.type_id == 47702).expect("web fixture");
    let bcs = fixtures.iter().find(|f| f.type_id == 49726).expect("BCS fixture");

    // Two 50MN MWDs (worst and best roll), one web, and the best-rolled
    // Ballistic Control System, which carries a gold bar.
    let mwd_worst = &mwd.modules[0];
    let mwd_best = mwd.modules.last().expect("modules");
    let web_module = &web.modules[0];
    let gold_module = bcs.modules.last().expect("modules");
    assert!(
        gold_module.expected.attributes.iter().any(|attribute| attribute.bar == 1),
        "fixture expectation: the BCS module has a gold bar",
    );

    // An extra module that stays unlisted (no contract): visible on the
    // all-modules page only.
    let mwd_unlisted = &mwd.modules[1];

    for (type_id, module) in [
        (mwd.type_id, mwd_worst),
        (mwd.type_id, mwd_best),
        (mwd.type_id, mwd_unlisted),
        (web.type_id, web_module),
        (bcs.type_id, gold_module),
    ] {
        ingest(&pool, &reference, type_id, module).await;
    }

    // For-sale state mirroring the legacy browse visibility, with the
    // spread needed by the price and contract filters.
    common::attach_contract(&pool, mwd_worst.module_id, 800_001, "item_exchange", 100_000_000.0, 1, 0, 0).await;
    common::attach_contract(&pool, mwd_best.module_id, 800_002, "auction", 500_000_000.0, 1, 2, 0).await;
    common::attach_contract(&pool, web_module.module_id, 800_003, "item_exchange", 200_000_000.0, 1, 1, 500).await;
    common::attach_contract(&pool, gold_module.module_id, 800_004, "item_exchange", 900_000_000.0, 1, 0, 0).await;
    sqlx::query("update modules set latest_contract_id = null where id = $1")
        .bind(mwd_unlisted.module_id)
        .execute(&pool)
        .await
        .expect("unlist module");

    let app = mutamarket::server::test_router().await;

    // Type scoping: only modules of the type, by id and by slug.
    let (status, body, _) = get(&app, "/api/modules/type/47408").await;
    assert_eq!(status, StatusCode::OK);
    let ids = data_ids(&body);
    assert!(ids.contains(&mwd_worst.module_id) && ids.contains(&mwd_best.module_id));
    assert!(!ids.contains(&web_module.module_id), "other types are excluded");
    assert!(!ids.contains(&gold_module.module_id));

    let (_, by_slug, _) = get(&app, "/api/modules/type/50mn-abyssal-microwarpdrive").await;
    assert_eq!(data_ids(&by_slug), ids, "slug resolves to the same type");

    // Sorting by roll quality, both directions.
    let (_, ascending, _) = get(&app, "/api/modules/type/47408/sort/fraction/asc").await;
    assert_eq!(data_ids(&ascending), vec![mwd_worst.module_id, mwd_best.module_id]);
    let (_, descending, _) = get(&app, "/api/modules/type/47408/sort/fraction/desc").await;
    assert_eq!(data_ids(&descending), vec![mwd_best.module_id, mwd_worst.module_id]);

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
    let mut keys: Vec<&str> = page.as_object().expect("object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["data", "links", "meta"]);
    let mut link_keys: Vec<&str> =
        page["links"].as_object().expect("links").keys().map(String::as_str).collect();
    link_keys.sort_unstable();
    assert_eq!(link_keys, ["first", "last", "next", "prev"]);
    let mut meta_keys: Vec<&str> =
        page["meta"].as_object().expect("meta").keys().map(String::as_str).collect();
    meta_keys.sort_unstable();
    assert_eq!(meta_keys, ["next_cursor", "path", "per_page", "prev_cursor"]);
    assert_eq!(page["links"]["first"], serde_json::Value::Null);
    assert_eq!(page["links"]["last"], serde_json::Value::Null);
    assert_eq!(page["meta"]["per_page"], serde_json::json!(100));
    assert_eq!(page["meta"]["path"], serde_json::json!("/api/modules/type/47408"));
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
    assert!(data_ids(&valued).is_empty(), "no estimates yet, so no matches");
    let (_, zero_bound, _) = get(&app, "/api/modules/type/47408/estimated-value/0-5000").await;
    assert_eq!(data_ids(&zero_bound).len(), 2, "zero lower bound disables the filter");

    // Unlisted modules are hidden from the browse pages but shown on the
    // all-modules page, like the legacy visibility split.
    let (_, listed_only, _) = get(&app, "/api/modules/type/47408").await;
    assert!(!data_ids(&listed_only).contains(&mwd_unlisted.module_id));
    let (status, all_cards, _) =
        get(&app, "/api/module-cards/type/50mn-abyssal-microwarpdrive?unlisted=true").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        all_cards
            .as_array()
            .expect("bare card array")
            .iter()
            .any(|module| module["id"] == serde_json::json!(mwd_unlisted.module_id)),
        "the all-modules set includes the unlisted module",
    );

    // Price sorting over the unified contract price, both directions.
    let (_, price_asc, _) = get(&app, "/api/modules/type/47408/sort/price/asc").await;
    assert_eq!(data_ids(&price_asc), vec![mwd_worst.module_id, mwd_best.module_id]);
    let (_, price_desc, _) = get(&app, "/api/modules/type/47408/sort/price/desc").await;
    assert_eq!(data_ids(&price_desc), vec![mwd_best.module_id, mwd_worst.module_id]);

    // Contract price bounds: a single number is a maximum, a range is
    // inclusive, and a zero lower bound disables the filter.
    let (_, max_bound, _) = get(&app, "/api/modules/type/47408/contract-price/300000000").await;
    assert_eq!(data_ids(&max_bound), vec![mwd_worst.module_id]);
    let (_, range_bound, _) =
        get(&app, "/api/modules/type/47408/contract-price/50000000-600000000").await;
    assert_eq!(data_ids(&range_bound).len(), 2);
    let (_, zero_bound_price, _) =
        get(&app, "/api/modules/type/47408/contract-price/0-100").await;
    assert_eq!(data_ids(&zero_bound_price).len(), 2, "zero lower bound disables the filter");

    // Contract type flags.
    let (_, auctions, _) = get(&app, "/api/modules/type/47408/auction").await;
    assert_eq!(data_ids(&auctions), vec![mwd_best.module_id]);
    let (_, exchanges, _) = get(&app, "/api/modules/type/47408/item-exchange").await;
    assert_eq!(data_ids(&exchanges), vec![mwd_worst.module_id]);

    // Single-item and without-other-items rules.
    let (_, single, _) = get(&app, "/api/modules/type/47408/no-multi-item-contracts").await;
    assert_eq!(data_ids(&single), vec![mwd_worst.module_id]);
    let (_, clean, _) = get(&app, "/api/modules/type/47408/without-other-items").await;
    assert_eq!(data_ids(&clean), vec![mwd_worst.module_id]);
    let (_, plex_ok, _) = get(&app, "/api/modules/type/47702/without-other-items").await;
    assert_eq!(
        data_ids(&plex_ok),
        vec![web_module.module_id],
        "one extra item is fine when it is asked-for PLEX",
    );

    // Legacy error semantics.
    let (status, body, _) = get(&app, "/api/modules/type/not-a-real-type-anywhere").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], serde_json::json!("Please provide a valid type."));

    let (status, body, _) = get(&app, "/api/modules/type/47408/meta-group/imaginary").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body["message"],
        serde_json::json!("You provided an invalid meta group: imaginary"),
    );

    let (status, body, _) = get(&app, "/api/modules/type/47408/attributes/notanattribute/5").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], serde_json::json!("Unknown attribute: notanattribute"));

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
    assert_eq!(cards[0], index["data"][0], "cards serialize like the index resource");

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
    assert_eq!(body["message"], serde_json::json!("Please provide a valid type."));
    let (status, body, _) =
        get(&app, "/api/module-cards/type/47408/attributes/notanattribute/5").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], serde_json::json!("Unknown attribute: notanattribute"));

    // The market stats strip payload.
    let (status, stats, _) = get(&app, "/api/module-stats").await;
    assert_eq!(status, StatusCode::OK);
    let mut stats_keys: Vec<&str> =
        stats.as_object().expect("stats object").keys().map(String::as_str).collect();
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
            "total_count",
        ],
    );
    assert!(stats["total_count"].as_i64().expect("count") >= 4, "the seeded modules count");

    // The filter panel resolves the type like the search does.
    let (status, panel, _) = get(&app, "/api/filter-panel/50mn-abyssal-microwarpdrive").await;
    assert_eq!(status, StatusCode::OK);
    let mut panel_keys: Vec<&str> =
        panel.as_object().expect("panel object").keys().map(String::as_str).collect();
    panel_keys.sort_unstable();
    assert_eq!(panel_keys, ["attributes", "type_id", "type_name"]);
    assert_eq!(panel["type_id"], serde_json::json!(47408));
    assert_eq!(panel["type_name"], serde_json::json!("50MN Abyssal Microwarpdrive"));
    let attributes = panel["attributes"].as_array().expect("attributes");
    assert!(!attributes.is_empty());
    for attribute in attributes {
        let mut keys: Vec<&str> =
            attribute.as_object().expect("attribute object").keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "attribute_id",
                "best",
                "display_name",
                "high_is_good",
                "name",
                "unit_display_name",
                "unit_name",
                "worst",
            ],
        );
    }

    let (status, body, _) = get(&app, "/api/filter-panel/not-a-real-type-anywhere").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], serde_json::json!("Please provide a valid type."));

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
