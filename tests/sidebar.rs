//! Behavior tests for the sidebar (the legacy `BookmarkController` and
//! the Advertisements/GearItems shared props): the bookmark round trip
//! with ownership, and the visible-rotation filters.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    pool
}

async fn send(
    app: &axum::Router,
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
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null), location)
}

#[tokio::test]
async fn bookmarks_and_rotations_round_trip() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;

    // Idempotent slate.
    sqlx::query("delete from users where name in ('Sidebar Tester', 'Sidebar Other')")
        .execute(&pool)
        .await
        .expect("clean users");
    sqlx::query("delete from advertisements").execute(&pool).await.expect("clean ads");
    sqlx::query("delete from gear_items").execute(&pool).await.expect("clean gear");

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Sidebar Tester') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    let session = create_session(&pool, user_id, None).await.expect("session");
    let other_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Sidebar Other') returning id")
            .fetch_one(&pool)
            .await
            .expect("other user");
    let other = create_session(&pool, other_id, None).await.expect("other session");

    // Guests: null bookmarks in the payload, login redirects on actions.
    let (status, body, _) = send(&app, Method::GET, "/api/sidebar", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let mut keys: Vec<&str> =
        body.as_object().expect("payload").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["advertisements", "bookmarks", "gear_items"]);
    assert!(body["bookmarks"].is_null());
    let (status, _, location) =
        send(&app, Method::POST, "/bookmarks", None, Some(json!({}))).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/login");

    // Validation and creation.
    let (status, body, _) =
        send(&app, Method::POST, "/bookmarks", Some(&session), Some(json!({ "name": "X" }))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["query"][0], json!("The query field is required."));
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/bookmarks",
        Some(&session),
        Some(json!({ "query": "/modules/type/47408", "name": "Webs" })),
    )
    .await;
    assert!(status.is_redirection(), "bookmark create redirects back: {status}");

    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", Some(&session), None).await;
    let bookmarks = body["bookmarks"].as_array().expect("bookmarks");
    assert_eq!(bookmarks.len(), 1);
    let mut keys: Vec<&str> =
        bookmarks[0].as_object().expect("bookmark").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["id", "name", "query", "type_id"]);
    assert_eq!(bookmarks[0]["name"], json!("Webs"));
    let bookmark_id = bookmarks[0]["id"].as_i64().expect("id");

    // Rename and ownership.
    let (status, body, _) = send(
        &app,
        Method::PUT,
        &format!("/bookmarks/{bookmark_id}"),
        Some(&other),
        Some(json!({ "name": "Steal" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("Forbidden."));
    let (status, _, _) = send(
        &app,
        Method::PUT,
        &format!("/bookmarks/{bookmark_id}"),
        Some(&session),
        Some(json!({ "name": "Webifiers" })),
    )
    .await;
    assert!(status.is_redirection());
    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", Some(&session), None).await;
    assert_eq!(body["bookmarks"][0]["name"], json!("Webifiers"));

    // Delete: strangers 403, the owner clears it.
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/bookmarks/{bookmark_id}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/bookmarks/{bookmark_id}"),
        Some(&session),
        None,
    )
    .await;
    assert!(status.is_redirection());
    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", Some(&session), None).await;
    assert_eq!(body["bookmarks"], json!([]));

    // The ad rotation honors the legacy visible() scope.
    sqlx::query(
        "insert into advertisements (name, image_url, link, active, size, priority, starts_at, expires_at)
         values ('Live', 'https://example.com/a.png', 'https://example.com', true, '250x300', 5, null, null),
                ('Inactive', null, null, false, '250x300', 9, null, null),
                ('Expired', null, null, true, '250x300', 9, null, now() - interval '1 day'),
                ('Upcoming', null, null, true, '250x300', 9, now() + interval '1 day', null),
                ('Second', null, null, true, '250x300', 1, null, null)",
    )
    .execute(&pool)
    .await
    .expect("seed ads");
    sqlx::query(
        "insert into gear_items (name, link, active, priority)
         values ('Mouse', 'https://example.com/mouse', true, 1),
                ('Hidden', 'https://example.com/hidden', false, 9)",
    )
    .execute(&pool)
    .await
    .expect("seed gear");

    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", None, None).await;
    let names: Vec<&str> = body["advertisements"]
        .as_array()
        .expect("ads")
        .iter()
        .filter_map(|ad| ad["name"].as_str())
        .collect();
    assert_eq!(names, ["Live", "Second"], "visible scope and priority order");
    let gear: Vec<&str> = body["gear_items"]
        .as_array()
        .expect("gear")
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert_eq!(gear, ["Mouse"]);
}
