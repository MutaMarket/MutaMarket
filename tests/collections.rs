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

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> =
        value.as_object().expect("a JSON object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}


/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
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
        &estimator_stub(),
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

    // The private collection is visible to its owner, 403 to others (with
    // slug binding by the trailing identifier, any name prefix), and the
    // show URL 404s for unknown identifiers.
    let (status, _, body) =
        send(&app, "GET", &format!("/api/collections/{slug}"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(sorted_keys(&page), ["collection", "modules"]);
    assert_eq!(
        sorted_keys(&page["collection"]),
        ["character_name", "description", "id", "modules_count", "name", "slug", "visibility"],
    );
    assert_eq!(page["collection"]["name"], json!("Prized Rolls"));
    assert_eq!(page["collection"]["character_name"], json!("Collector One"));
    assert_eq!(page["collection"]["visibility"], json!("private"));
    let (status, _, body) =
        send(&app, "GET", &format!("/api/collections/{renamed_slug}"), Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(error["message"], json!("This collection is private."));
    let (status, _, body) = send(&app, "GET", "/api/collections/unknown-zzzz", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(error["message"], json!("Collection not found"));

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
        send(&app, "GET", &format!("/api/collections/{renamed_slug}"), Some(&other), None).await;
    assert_eq!(status, StatusCode::OK);

    // The JSON index carries the card shape; search narrows by name.
    let (status, _, body) = send(&app, "GET", "/api/collections", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let cards: serde_json::Value = serde_json::from_str(&body).expect("json");
    let card = cards
        .as_array()
        .expect("card array")
        .iter()
        .find(|card| card["name"] == json!("Shiny Rolls"))
        .expect("the public collection lists")
        .clone();
    assert_eq!(
        sorted_keys(&card),
        ["character_name", "description", "id", "modules_count", "name", "slug", "visibility"],
    );
    assert_eq!(card["modules_count"], json!(1));
    assert_eq!(card["character_name"], json!("Collector One"));
    let (status, _, body) =
        send(&app, "GET", "/api/collections?search=no-collection-matches-this", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let cards: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(
        !cards.as_array().expect("card array").iter().any(|card| card["name"] == json!("Shiny Rolls")),
        "search narrows the index",
    );

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
    let (status, _, _) =
        send(&app, "GET", &format!("/api/collections/{slug}"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Character data: description updates are owner-only; the page data
    // itself is pinned below through the JSON endpoints.
    // The JSON character endpoints: the index lists only characters with
    // public ownerships, the show payload mirrors the page.
    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id) values (910001, $1)
         on conflict do nothing",
    )
    .bind(module.module_id)
    .execute(&pool)
    .await
    .expect("seed public ownership");
    let (status, _, body) = send(&app, "GET", "/api/characters?search=Collector", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let cards: serde_json::Value = serde_json::from_str(&body).expect("json");
    let card = cards
        .as_array()
        .expect("card array")
        .iter()
        .find(|card| card["id"] == json!(910001))
        .expect("Collector One lists with a public ownership")
        .clone();
    assert_eq!(
        sorted_keys(&card),
        ["corporation_id", "description", "has_premium", "id", "modules_count", "name", "slug"],
    );
    assert_eq!(card["name"], json!("Collector One"));
    assert_eq!(card["slug"], json!("collector-one-910001"));
    assert_eq!(card["modules_count"], json!(1));
    assert!(
        !cards.as_array().expect("card array").iter().any(|card| card["id"] == json!(910002)),
        "Collector Two has no public ownership and stays unlisted",
    );

    let (status, _, body) =
        send(&app, "GET", "/api/characters/collector-one-910001", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(sorted_keys(&page), ["character", "modules"]);
    assert_eq!(
        sorted_keys(&page["character"]),
        ["corporation_id", "description", "has_premium", "id", "modules_count", "name", "slug"],
    );
    assert_eq!(page["character"]["name"], json!("Collector One"));
    assert_eq!(
        page["modules"].as_array().expect("modules").len(),
        1,
        "the publicly owned module renders on the page",
    );
    assert_eq!(page["modules"][0]["id"], json!(module.module_id));

    // The filter grammar applies scoped to the page: a matching type
    // keeps the module, a different type filters it out, and the
    // `created` option switches to the creations scope.
    let type_query = format!("q=type/{}", fixture.type_id);
    let (status, _, body) = send(
        &app,
        "GET",
        &format!("/api/characters/collector-one-910001?{type_query}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["modules"].as_array().expect("modules").len(), 1);
    let (_, _, body) = send(
        &app,
        "GET",
        "/api/characters/collector-one-910001?q=type/47702",
        None,
        None,
    )
    .await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["modules"].as_array().expect("modules").len(), 0, "other types filter out");
    let (_, _, body) = send(
        &app,
        "GET",
        "/api/characters/collector-one-910001?q=created",
        None,
        None,
    )
    .await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    let created_ids: Vec<i64> = page["modules"]
        .as_array()
        .expect("modules")
        .iter()
        .filter_map(|entry| entry["id"].as_i64())
        .collect();
    assert_eq!(
        created_ids.contains(&module.module_id),
        module.creator_id == 910001,
        "the created scope lists exactly the character's creations",
    );

    let (status, _, body) = send(&app, "GET", "/api/characters/999999999", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(error["message"], json!("Character not found"));
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
