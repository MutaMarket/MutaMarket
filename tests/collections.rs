//! Behavior tests for the collections backend: CRUD through the real
//! router with sessions, slug binding, the visibility policy, and the
//! character pages' status contracts.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use std::path::Path;
use tower::ServiceExt;

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String, String) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        request = request.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("valid request");

    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, location, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn collections_crud_and_policy() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables).await.expect("seed");
    let reference = ReferenceData::from_tables(tables);

    // A module to collect.
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

    // Two users with characters; idempotent across runs.
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![910_001_i64, 910_002_i64])
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Collector One", "Collector Two"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let mut sessions = Vec::new();
    for (user_name, character_id) in [("Collector One", 910_001_i64), ("Collector Two", 910_002_i64)]
    {
        let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
            .bind(user_name)
            .fetch_one(&pool)
            .await
            .expect("user");
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(character_id)
            .bind(user_name)
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("character");
        let session =
            mutamarket::auth::session::create_session(&pool, user_id, Some(character_id))
                .await
                .expect("session");
        sessions.push(session);
    }
    let (owner, other) = (sessions[0].clone(), sessions[1].clone());

    let app = mutamarket::server::test_router().await;

    // Guests are redirected to login.
    let (status, location, _) =
        send(&app, "POST", "/collections", None, Some(json!({"name": "x"}))).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/login");

    // Invalid payloads answer the Laravel 422 shape.
    let (status, _, body) = send(
        &app,
        "POST",
        "/collections",
        Some(&owner),
        Some(json!({"name": "", "visibility": "bogus"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(errors["message"], json!("The given data was invalid."));
    assert!(errors["errors"]["name"].is_array());
    assert!(errors["errors"]["visibility"].is_array());

    // Create: redirects to the slugged show URL.
    let (status, location, _) = send(
        &app,
        "POST",
        "/collections",
        Some(&owner),
        Some(json!({"name": "Prized Rolls", "visibility": "private"})),
    )
    .await;
    assert!(status.is_redirection(), "create redirects, got {status}");
    assert!(
        location.starts_with("/collections/prized-rolls-"),
        "redirect carries the slug: {location}",
    );
    let slug = location.trim_start_matches("/collections/").to_owned();

    // Slug binding resolves by the trailing identifier, any name prefix.
    let renamed_slug = format!("whatever-{}", slug.rsplit('-').next().unwrap());

    // The private collection is visible to its owner, 403 to others, and
    // the show URL 404s for unknown identifiers.
    let (status, _, _) = send(&app, "GET", &format!("/collections/{slug}"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) =
        send(&app, "GET", &format!("/collections/{renamed_slug}"), Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(&app, "GET", "/collections/unknown-zzzz", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Add a module (owner only).
    let collection_id: i64 =
        sqlx::query_scalar("select id from collections where identifier = $1")
            .bind(slug.rsplit('-').next().unwrap())
            .fetch_one(&pool)
            .await
            .expect("collection row");
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-modules",
        Some(&other),
        Some(json!({"collection_id": collection_id, "module_id": module.module_id})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "non-owners cannot add modules");
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-modules",
        Some(&owner),
        Some(json!({"collection_id": collection_id, "module_id": module.module_id})),
    )
    .await;
    assert!(status.is_redirection(), "adding a module redirects back, got {status}");
    let linked: i64 =
        sqlx::query_scalar("select count(*) from collection_modules where collection_id = $1")
            .bind(collection_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(linked, 1);

    // Update: visibility flips to public, new name lands in the redirect.
    let (status, location, _) = send(
        &app,
        "PUT",
        &format!("/collections/{slug}"),
        Some(&owner),
        Some(json!({"name": "Shiny Rolls", "visibility": "public"})),
    )
    .await;
    assert!(status.is_redirection());
    assert!(location.starts_with("/collections/shiny-rolls-"), "renamed slug: {location}");

    // Now public: the other user can view it and it lists on the index.
    let (status, _, _) =
        send(&app, "GET", &format!("/collections/{renamed_slug}"), Some(&other), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, body) = send(&app, "GET", "/collections", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Shiny Rolls"), "public index lists the collection");

    // Remove all modules, then delete; the collection is gone.
    let (status, _, _) = send(
        &app,
        "DELETE",
        "/collection-modules/all",
        Some(&owner),
        Some(json!({"collection_id": collection_id})),
    )
    .await;
    assert!(status.is_redirection());
    let (status, location, _) =
        send(&app, "DELETE", &format!("/collections/{slug}"), Some(&owner), None).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/collections");
    let (status, _, _) = send(&app, "GET", &format!("/collections/{slug}"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Character pages: known character renders, its description updates,
    // unknown 404s, foreign characters cannot be edited.
    let (status, _, body) = send(&app, "GET", "/characters/collector-one-910001", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Collector One"));
    let (status, _, _) = send(
        &app,
        "PUT",
        "/characters/collector-one-910001",
        Some(&other),
        Some(json!({"description": "hi"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(
        &app,
        "PUT",
        "/characters/collector-one-910001",
        Some(&owner),
        Some(json!({"description": "Roll dealer"})),
    )
    .await;
    assert!(status.is_redirection());
    let description: Option<String> =
        sqlx::query_scalar("select description from characters where id = 910001")
            .fetch_one(&pool)
            .await
            .expect("description");
    assert_eq!(description.as_deref(), Some("Roll dealer"));
}
