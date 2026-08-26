//! Behavior tests for the appraise flow (`POST /modules`, the legacy
//! `ModuleController::store` + `GetModuleJob`): a pasted in-game link
//! resolves, ingests through ESI and redirects to the module's show
//! page; bad input answers the legacy failure text.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Path as AxumPath;
use axum::http::{Request, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use http_body_util::BodyExt;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");

    (pool, ReferenceData::from_tables(tables))
}

fn app(pool: &PgPool, reference: ReferenceData, esi_url: &str) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new(esi_url),
        SsoClient::new("http://127.0.0.1:9", "client", "secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(reference),
        None,
    )
}

/// A mock ESI serving one dynamic item.
async fn start_mock(fixture_type_id: i64, module: &common::ModuleFixture) -> String {
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
    });
    let module_item = module.module_id;

    let router = Router::new().route(
        "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
        get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
            let payload = payload.clone();
            async move {
                if type_id == fixture_type_id && item_id == module_item {
                    return Json(payload).into_response();
                }
                StatusCode::NOT_FOUND.into_response()
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

async fn post_store(
    app: &Router,
    body: serde_json::Value,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/modules")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, location, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

#[tokio::test]
async fn a_pasted_link_ingests_the_module_and_redirects_to_it() {
    let (pool, reference) = setup().await;

    // The fourth fixture keeps this suite's module distinct from the
    // ones the other suites ingest.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[3];
    let module = &fixture.modules[0];

    sqlx::query("delete from modules where id = $1")
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("clean module");

    let esi_url = start_mock(fixture.type_id, module).await;
    let app = app(&pool, reference, &esi_url);

    let message = format!(
        "<url=showinfo:{}//{}>Abyssal Module</url>",
        fixture.type_id, module.module_id,
    );
    let (status, location, _) = post_store(&app, json!({ "message": message })).await;
    assert!(status.is_redirection(), "success redirects: {status}");
    assert_eq!(location.as_deref(), Some(format!("/modules/{}", module.module_id).as_str()));

    let stored: Option<i64> = sqlx::query_scalar("select id from modules where id = $1")
        .bind(module.module_id)
        .fetch_optional(&pool)
        .await
        .expect("module row");
    assert_eq!(stored, Some(module.module_id));

    // A repeat post is a no-op success, like the legacy job's
    // already-known short-circuit.
    let (status, location, _) = post_store(&app, json!({ "message": message })).await;
    assert!(status.is_redirection());
    assert_eq!(location.as_deref(), Some(format!("/modules/{}", module.module_id).as_str()));

    // The explicit pair works without a message (the legacy
    // required_without rules).
    let (status, _, _) = post_store(
        &app,
        json!({ "type_id": fixture.type_id, "item_id": module.module_id }),
    )
    .await;
    assert!(status.is_redirection());
}

#[tokio::test]
async fn bad_input_answers_the_legacy_failure_text() {
    let (pool, reference) = setup().await;
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[3];
    let module = &fixture.modules[0];
    let esi_url = start_mock(fixture.type_id, module).await;
    let app = app(&pool, reference, &esi_url);

    // A message without a link fails with the legacy notification body.
    let (status, _, body) = post_store(&app, json!({ "message": "no link here" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["message"],
        json!(
            "We were unable to add the module to the database. \
             Please check your input and try again."
        ),
    );

    // An unknown item (ESI 404) fails the same way.
    let (status, _, body) =
        post_store(&app, json!({ "message": "<url=showinfo:47740//123456789>x</url>" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["message"].as_str().expect("message").starts_with("We were unable"));

    // Nothing at all: the required_without validation.
    let (status, _, body) = post_store(&app, json!({})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["message"],
        json!("The message field is required when item id is not present."),
    );
}
