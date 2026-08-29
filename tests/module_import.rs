//! Behavior tests for module import via `POST /api/modules`, with ESI
//! replaced by a local mock server serving fixture dogma data.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use tower::ServiceExt;

/// Serves the fixture module's dynamic-item dogma the way ESI would, and
/// 404 for anything else.
async fn start_mock_esi(fixture_type_id: i64, module: &common::ModuleFixture) -> String {
    let payload = json!({
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
        "dogma_effects": [],
    });

    let known = (fixture_type_id, module.module_id);

    let app = Router::new().route(
        "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
        get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
            let payload = payload.clone();
            async move {
                if (type_id, item_id) == known {
                    Json(payload).into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock ESI address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock ESI");
    });

    format!("http://{address}")
}

async fn post_json(
    app: &Router,
    path: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("valid request"),
        )
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, body)
}

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

#[tokio::test]
async fn imports_modules_from_esi_through_the_api() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = Arc::new(ReferenceData::from_tables(tables));

    // Use the second fixture file so this test does not interfere with the
    // module the API read tests ingest, and clear it to force the ESI path.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[1];
    let module = &fixture.modules[0];

    sqlx::query("delete from modules where id = $1")
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("clear module");

    let esi_url = start_mock_esi(fixture.type_id, module).await;

    let app = mutamarket::server::router(
        pool.clone(),
        EsiClient::new(&esi_url),
        mutamarket::auth::sso::SsoClient::from_env(),
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator_stub(),
        reference,
        None,
    );

    // Import by explicit type and item id.
    let (status, body) = post_json(
        &app,
        "/api/modules",
        json!({ "type_id": fixture.type_id, "item_id": module.module_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["id"], json!(module.module_id));
    assert_eq!(
        body["data"]["mutated_attributes"].as_array().map(Vec::len),
        Some(module.expected.attributes.len()),
    );

    let average: Option<f64> =
        sqlx::query_scalar("select average_fraction from modules where id = $1")
            .bind(module.module_id)
            .fetch_one(&pool)
            .await
            .expect("average fraction");
    assert!(
        average.is_some_and(|average| common::matches(module.expected.average_fraction, average))
    );

    // Re-submitting via an item link message returns the existing module
    // without refetching (the mock would still serve it, but the early
    // return path is what the legacy job does).
    let (status, body) = post_json(
        &app,
        "/api/modules",
        json!({
            "message": format!(
                "<url=showinfo:{}//{}>my roll</url>",
                fixture.type_id, module.module_id,
            ),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(module.module_id));

    // Empty payload: legacy-shaped validation error.
    let (status, body) = post_json(&app, "/api/modules", json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["errors"].is_object());
    assert!(body["message"].is_string());

    // A message without an item link fails like the legacy controller.
    let (status, body) =
        post_json(&app, "/api/modules", json!({ "message": "no link here" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], json!("Failed to add module!"));

    // An item ESI does not know fails with the legacy message.
    let (status, body) = post_json(
        &app,
        "/api/modules",
        json!({ "type_id": fixture.type_id, "item_id": 999_999_999_999_i64 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], json!("Failed to add module!"));
}
