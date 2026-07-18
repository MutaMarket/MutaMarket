//! Behavior tests for the module JSON API against real data: seed the
//! fixture reference, ingest a known module, and exercise show and index
//! through the full router.
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

async fn get_json(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, json)
}

#[tokio::test]
async fn module_api_serves_ingested_modules() {
    let pool = db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    // Ingest the first module of the first fixture file: a 50MN Abyssal
    // Microwarpdrive (type 47408).
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let module = &fixture.modules[0];

    process_module(
        &pool,
        &reference,
        fixture.type_id,
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

    let app = mutamarket::server::test_router().await;

    // Show by bare item id.
    let (status, body) = get_json(&app, &format!("/api/modules/{}", module.module_id)).await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    assert_eq!(data["id"], serde_json::json!(module.module_id));
    assert_eq!(data["type_id"], serde_json::json!(fixture.type_id));
    assert_eq!(data["source_type_id"], serde_json::json!(module.source_type_id));
    assert_eq!(data["mutaplasmid_id"], serde_json::json!(module.mutaplasmid_id));
    assert_eq!(
        data["attributes"].as_array().map(Vec::len),
        Some(module.expected.attributes.len()),
    );

    let slug = data["slug"].as_str().expect("slug present").to_owned();
    assert!(slug.ends_with(&module.module_id.to_string()));

    // A rolled attribute matches its expected computed values.
    let expected_first = &module.expected.attributes[0];
    let attribute = data["attributes"]
        .as_array()
        .expect("attributes array")
        .iter()
        .find(|attribute| attribute["attribute_id"] == serde_json::json!(expected_first.attribute_id))
        .expect("expected attribute present");
    assert!(common::matches(
        expected_first.fraction,
        attribute["fraction"].as_f64().expect("fraction"),
    ));

    // Show by slug.
    let (status, body) = get_json(&app, &format!("/api/modules/{slug}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], serde_json::json!(module.module_id));

    // Unknown module id.
    let (status, body) = get_json(&app, "/api/modules/does-not-exist-999999999999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["message"].is_string());

    // Type-scoped index by id and by slug contains the module.
    for type_query in [
        format!("/api/modules/type/{}", fixture.type_id),
        "/api/modules/type/50mn-abyssal-microwarpdrive".to_owned(),
    ] {
        let (status, body) = get_json(&app, &type_query).await;
        assert_eq!(status, StatusCode::OK, "{type_query}");
        let ids: Vec<i64> = body["data"]
            .as_array()
            .expect("data array")
            .iter()
            .filter_map(|module| module["id"].as_i64())
            .collect();
        assert!(ids.contains(&module.module_id), "{type_query}: {ids:?}");
    }

    // The index without a type option rejects, like the legacy API.
    let (status, body) = get_json(&app, "/api/modules").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], serde_json::json!("Please provide a valid type."));

    // Estimator statistics serve JSON (empty until the estimator lands).
    let (status, body) = get_json(&app, "/api/estimator-statistics").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}
