//! Behavior test for `GET /api/abyssal-type-statistics`: seed the fixture
//! reference plus the computed abyssal aggregates and verify the endpoint
//! emits the exact legacy resource shape (bare array, no `data` wrapper,
//! key-set parity at every nesting level).
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use mutamarket::sde::statistics::compute_abyssal_statistics;
use tower::ServiceExt;

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

#[tokio::test]
async fn abyssal_type_statistics_match_the_legacy_resource() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let mut tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    tables.abyssal_statistics = compute_abyssal_statistics(&tables);
    seed_reference(&pool, &tables).await.expect("seed reference tables");

    let app = mutamarket::server::test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/abyssal-type-statistics")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("application/json"),
        "unexpected content type {content_type}",
    );

    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");

    // The legacy controller returns the resource collection itself, so the
    // response is a bare array without the usual `data` wrapper.
    let rows = body.as_array().expect("a bare JSON array");
    assert_eq!(rows.len(), tables.abyssal_statistics.len());
    assert!(!rows.is_empty(), "no statistics seeded");

    // Exact key sets at every nesting level, for every row. `meta_level`
    // is absent from the type (whenHas checks attributes, not relations)
    // and no timestamps are emitted.
    let mut rows_with_unit = 0usize;
    let mut rows_with_meta_group = 0usize;
    for row in rows {
        assert_eq!(
            sorted_keys(row),
            vec![
                "attribute",
                "attribute_id",
                "best",
                "high_is_good",
                "id",
                "is_derived",
                "is_virtual",
                "type",
                "type_id",
                "worst",
            ],
        );
        assert_eq!(
            sorted_keys(&row["attribute"]),
            vec!["display_name", "high_is_good", "id", "is_derived", "name", "unit"],
        );
        if !row["attribute"]["unit"].is_null() {
            assert_eq!(sorted_keys(&row["attribute"]["unit"]), vec![
                "display_name",
                "id",
                "name",
            ]);
            rows_with_unit += 1;
        }
        assert_eq!(sorted_keys(&row["type"]), vec![
            "id",
            "meta_group",
            "meta_group_id",
            "name",
            "published",
        ]);
        if !row["type"]["meta_group"].is_null() {
            rows_with_meta_group += 1;
        }

        // The top-level is_derived mirrors the attribute's flag.
        assert_eq!(row["is_derived"], row["attribute"]["is_derived"]);
    }
    assert!(rows_with_unit > 0, "no attribute with a unit exercised");
    assert!(rows_with_meta_group > 0, "no type with a meta group exercised");

    // Rows come back ordered by id and mirror the seeded aggregates.
    for (row, expected) in rows.iter().zip(&tables.abyssal_statistics) {
        assert_eq!(row["id"], serde_json::json!(expected.id));
        assert_eq!(row["type_id"], serde_json::json!(expected.type_id));
        assert_eq!(row["attribute_id"], serde_json::json!(expected.attribute_id));
        assert_eq!(row["best"], serde_json::json!(expected.best));
        assert_eq!(row["worst"], serde_json::json!(expected.worst));
        assert_eq!(row["high_is_good"], serde_json::json!(expected.high_is_good));
        assert_eq!(row["is_virtual"], serde_json::json!(expected.is_virtual));
    }

    // Spot-check the joined relations of the first row against the tables.
    let first = &tables.abyssal_statistics[0];
    let attribute = tables
        .attributes
        .iter()
        .find(|attribute| attribute.id == first.attribute_id)
        .expect("attribute exists");
    let abyssal_type = tables
        .types
        .iter()
        .find(|row| row.id == first.type_id)
        .expect("type exists");
    assert_eq!(rows[0]["attribute"]["id"], serde_json::json!(attribute.id));
    assert_eq!(rows[0]["attribute"]["name"], serde_json::json!(attribute.name));
    assert_eq!(
        rows[0]["attribute"]["display_name"],
        serde_json::json!(attribute.display_name),
    );
    assert_eq!(
        rows[0]["attribute"]["high_is_good"],
        serde_json::json!(attribute.high_is_good),
    );
    assert_eq!(rows[0]["is_derived"], serde_json::json!(attribute.derived));
    assert_eq!(rows[0]["type"]["id"], serde_json::json!(abyssal_type.id));
    assert_eq!(rows[0]["type"]["name"], serde_json::json!(abyssal_type.name));
    assert_eq!(
        rows[0]["type"]["published"],
        serde_json::json!(abyssal_type.published),
    );
}
