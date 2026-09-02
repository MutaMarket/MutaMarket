//! Behavior tests for the workbench (the legacy `WorkbenchController`
//! family): the per-user set, the no-op duplicate add, ownership on
//! delete, the shared invitation link, and the collection conversion.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const BENCH_CHARACTER: i64 = 990_600_001;
const GUEST_CHARACTER: i64 = 990_600_002;

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables)
        .await
        .expect("seed");
    (pool, ReferenceData::from_tables(tables))
}

fn app(pool: &PgPool, reference: ReferenceData) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new(
            "http://127.0.0.1:9",
            "client",
            "secret",
            "http://test/eve/callback",
        ),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(reference),
        None,
    )
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, String) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("request");
    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        location,
    )
}

#[tokio::test]
async fn the_workbench_round_trips_like_the_legacy_controllers() {
    let (pool, reference) = setup().await;

    // Two fixture modules to bench.
    let fixtures = common::load_module_fixtures();
    let mut module_ids = Vec::new();
    for fixture_index in [4usize, 5] {
        let fixture = &fixtures[fixture_index];
        let module = &fixture.modules[0];
        process_module(
            &pool,
            &reference,
            &Estimator::new(),
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
        module_ids.push(module.module_id);
    }

    for character in [BENCH_CHARACTER, GUEST_CHARACTER] {
        sqlx::query("delete from users where id in (select user_id from characters where id = $1)")
            .bind(character)
            .execute(&pool)
            .await
            .expect("clean user");
        sqlx::query("delete from characters where id = $1")
            .bind(character)
            .execute(&pool)
            .await
            .expect("clean character");
    }

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Bench Tester') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Bench Tester', $2)")
        .bind(BENCH_CHARACTER)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("character");
    let session = create_session(&pool, user_id, Some(BENCH_CHARACTER))
        .await
        .expect("session");
    sqlx::query("delete from collections where character_id = $1")
        .bind(BENCH_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean collections");

    let app = app(&pool, reference);

    // Guests: actions redirect to login, the api answers 401, the
    // shared page is public.
    let (status, _, location) = send(
        &app,
        Method::POST,
        "/workbench-modules",
        None,
        Some(json!({})),
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(location, "/login");
    let (status, body, _) = send(&app, Method::GET, "/api/workbench", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], json!("Unauthenticated."));
    let shared_path = format!("/api/workbench-page/{}/{}", module_ids[0], module_ids[1]);
    let (status, body, _) = send(&app, Method::GET, &shared_path, None, None).await;
    assert_eq!(status, StatusCode::OK);
    let shared = body.as_array().expect("shared modules");
    assert_eq!(shared.len(), 2);
    crate::common::assert_default_module_keys(&shared[0], false, &[]);

    // Adding: once, then a silent no-op duplicate.
    for _ in 0..2 {
        let (status, _, _) = send(
            &app,
            Method::POST,
            "/workbench-modules",
            Some(&session),
            Some(json!({ "module_id": module_ids[0] })),
        )
        .await;
        assert!(
            status.is_redirection(),
            "workbench add redirects back: {status}"
        );
    }
    let (status, body, _) = send(&app, Method::GET, "/api/workbench", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    let bench = body.as_array().expect("workbench");
    assert_eq!(bench.len(), 1, "the duplicate add is a no-op");
    assert_eq!(bench[0]["module"]["id"], json!(module_ids[0]));
    crate::common::assert_default_module_keys(&bench[0]["module"], true, &[]);
    let entry_id = bench[0]["id"].as_i64().expect("workbench module id");

    // Only the owner may remove an entry; the legacy answer is the
    // "Unauthorized!" text.
    let other_user: i64 =
        sqlx::query_scalar("insert into users (name) values ('Bench Guest') returning id")
            .fetch_one(&pool)
            .await
            .expect("other user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Bench Guest', $2)")
        .bind(GUEST_CHARACTER)
        .bind(other_user)
        .execute(&pool)
        .await
        .expect("other character");
    let other = create_session(&pool, other_user, Some(GUEST_CHARACTER))
        .await
        .expect("session");
    let (status, body, _) = send(
        &app,
        Method::DELETE,
        &format!("/workbench-modules/{entry_id}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("Unauthorized!"));

    // The invitation accept imports the shared set without duplicates.
    let (status, _, _) = send(
        &app,
        Method::POST,
        &format!("/workbench/{}/{}", module_ids[0], module_ids[1]),
        Some(&session),
        None,
    )
    .await;
    assert!(status.is_redirection());
    let (_, body, _) = send(&app, Method::GET, "/api/workbench", Some(&session), None).await;
    assert_eq!(body.as_array().expect("workbench").len(), 2);

    // The collection conversion: a private Workbench Collection with
    // both modules, landing on its page.
    let (status, _, location) = send(
        &app,
        Method::POST,
        "/workbench-collections",
        Some(&session),
        None,
    )
    .await;
    assert!(status.is_redirection(), "{status}");
    assert!(
        location.starts_with("/collections/workbench-collection-"),
        "{location}"
    );
    let (linked, visibility): (i64, String) = sqlx::query_as(
        "select (select count(*) from collection_modules where collection_id = c.id),
                c.visibility
         from collections c where c.character_id = $1
         order by c.id desc limit 1",
    )
    .bind(BENCH_CHARACTER)
    .fetch_one(&pool)
    .await
    .expect("collection");
    assert_eq!(linked, 2);
    assert_eq!(visibility, "private");

    // Clear all empties the bench.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        "/workbench-modules/all",
        Some(&session),
        None,
    )
    .await;
    assert!(status.is_redirection());
    let (_, body, _) = send(&app, Method::GET, "/api/workbench", Some(&session), None).await;
    assert_eq!(body, json!([]));
}
