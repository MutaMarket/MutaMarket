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
    let pool = db::test_pool()
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
    assert_eq!(data["type"]["id"], serde_json::json!(fixture.type_id));
    assert_eq!(data["source_type"]["id"], serde_json::json!(module.source_type_id));
    assert_eq!(data["mutaplasmid"]["id"], serde_json::json!(module.mutaplasmid_id));
    assert_eq!(
        data["mutated_attributes"].as_array().map(Vec::len),
        Some(module.expected.attributes.len()),
    );

    let slug = data["slug"].as_str().expect("slug present").to_owned();
    assert!(slug.ends_with(&module.module_id.to_string()));

    // A rolled attribute matches its expected computed values.
    let expected_first = &module.expected.attributes[0];
    let attribute = data["mutated_attributes"]
        .as_array()
        .expect("attributes array")
        .iter()
        .find(|attribute| attribute["id"] == serde_json::json!(expected_first.attribute_id))
        .expect("expected attribute present");
    assert!(common::matches(
        expected_first.fraction,
        attribute["fraction"].as_f64().expect("fraction"),
    ));

    // Exact key parity with the legacy ModuleResource for guests with the
    // default relations: every key legacy emits, nothing missing.
    fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("a JSON object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        keys
    }

    assert_eq!(
        sorted_keys(data),
        [
            "average_fraction",
            "contract",
            "creator",
            "estimated_value",
            "estimated_value_updated_at",
            "id",
            "mutaplasmid",
            "mutated_attributes",
            "public_asset",
            "slug",
            "source_type",
            "type",
        ],
        "module key set diverges from the legacy resource",
    );
    assert_eq!(sorted_keys(&data["type"]), ["id", "name"]);
    assert_eq!(sorted_keys(&data["mutaplasmid"]), ["id", "name"]);
    assert_eq!(
        sorted_keys(&data["source_type"]),
        ["id", "meta_group", "meta_group_id", "name", "published"],
    );
    assert_eq!(
        sorted_keys(&data["creator"]),
        ["corporation_id", "description", "has_premium", "id", "name", "slug"],
    );
    assert!(
        data["source_type"]["meta_group"].is_string(),
        "meta group name resolves: {}",
        data["source_type"]["meta_group"],
    );

    // Feature keys owned by unported milestones are present and null, like
    // legacy's loaded-but-empty relations.
    assert!(data["contract"].is_null());
    assert!(data["public_asset"].is_null());
    assert!(data["estimated_value"].is_null());
    assert!(data["estimated_value_updated_at"].is_null());

    // Attribute rows carry the exact legacy MutatedAttributeResource keys
    // (plus our server-computed type_band).
    assert_eq!(
        sorted_keys(attribute),
        [
            "bar",
            "base_value",
            "display_name",
            "fraction",
            "fraction_absolute",
            "fraction_type",
            "id",
            "is_derived",
            "is_virtual",
            "name",
            "type_band",
            "unit",
            "value",
        ],
        "attribute key set diverges from the legacy resource",
    );
    let unit_attribute = data["mutated_attributes"]
        .as_array()
        .expect("attributes array")
        .iter()
        .find(|attribute| attribute["unit"].is_object())
        .expect("an attribute with a unit");
    assert_eq!(sorted_keys(&unit_attribute["unit"]), ["display_name", "id", "name"]);

    // Show by slug.
    let (status, body) = get_json(&app, &format!("/api/modules/{slug}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], serde_json::json!(module.module_id));

    // Unknown module id.
    let (status, body) = get_json(&app, "/api/modules/does-not-exist-999999999999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["message"].is_string());

    // The index lists for-sale modules; give ours a live contract (after
    // the parity assertions above saw the loaded-but-empty null).
    common::attach_contract(
        &pool,
        module.module_id,
        800_201,
        "item_exchange",
        275_000_000.0,
        1,
        0,
        0,
    )
    .await;

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
