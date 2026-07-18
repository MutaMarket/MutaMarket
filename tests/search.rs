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
    let pool = db::connect()
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

    for (type_id, module) in [
        (mwd.type_id, mwd_worst),
        (mwd.type_id, mwd_best),
        (web.type_id, web_module),
        (bcs.type_id, gold_module),
    ] {
        ingest(&pool, &reference, type_id, module).await;
    }

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

    // The browser page applies the same search: the type page shows only
    // that type's modules.
    let (status, _, page) = get(&app, "/modules/type/50mn-abyssal-microwarpdrive").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains(&format!("-{}", mwd_worst.module_id)));
    assert!(page.contains(&format!("-{}", mwd_best.module_id)));
    assert!(
        !page.contains(&format!("-{}", web_module.module_id)),
        "the web module does not appear on the MWD type page",
    );

    let (status, _, page) = get(&app, "/modules/type/definitely-unknown-type-slug").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(page.contains("Please provide a valid type."));
}
