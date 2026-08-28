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

    // Scoped to this test's seeds: the management round-trip tests run
    // in parallel on the same tables and briefly rotate their own rows.
    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", None, None).await;
    let seeded_ads = ["Live", "Inactive", "Expired", "Upcoming", "Second"];
    let names: Vec<&str> = body["advertisements"]
        .as_array()
        .expect("ads")
        .iter()
        .filter_map(|ad| ad["name"].as_str())
        .filter(|name| seeded_ads.contains(name))
        .collect();
    assert_eq!(names, ["Live", "Second"], "visible scope and priority order");
    let seeded_gear = ["Mouse", "Hidden"];
    let gear: Vec<&str> = body["gear_items"]
        .as_array()
        .expect("gear")
        .iter()
        .filter_map(|item| item["name"].as_str())
        .filter(|name| seeded_gear.contains(name))
        .collect();
    assert_eq!(gear, ["Mouse"]);
}

#[tokio::test]
async fn advertisement_management_is_admin_gated_and_round_trips() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;

    sqlx::query("delete from users where name in ('Ad Admin', 'Ad Peasant')")
        .execute(&pool)
        .await
        .expect("clean users");
    sqlx::query("delete from advertisements where name like 'MGMT %'")
        .execute(&pool)
        .await
        .expect("clean ads");
    let admin_id: i64 = sqlx::query_scalar(
        "insert into users (name, is_admin) values ('Ad Admin', true) returning id",
    )
    .fetch_one(&pool)
    .await
    .expect("admin");
    let admin = create_session(&pool, admin_id, None).await.expect("session");
    let peasant_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Ad Peasant') returning id")
            .fetch_one(&pool)
            .await
            .expect("peasant");
    let peasant = create_session(&pool, peasant_id, None).await.expect("session");

    // Gating: guests 401, non-admins 403.
    let (status, _, _) = send(&app, Method::GET, "/api/admin/advertisements", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body, _) =
        send(&app, Method::GET, "/api/admin/advertisements", Some(&peasant), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("Forbidden."));

    // Validation mirrors the legacy rules.
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/api/admin/advertisements",
        Some(&admin),
        Some(json!({ "name": "MGMT Missing image" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["image_url"][0], json!("The image url field is required."));

    // Create, list with the derived status, toggle, update, delete.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/api/admin/advertisements",
        Some(&admin),
        Some(json!({
            "name": "MGMT Banner",
            "image_url": "https://example.com/banner.png",
            "link": "https://example.com",
            "priority": 3,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body, _) =
        send(&app, Method::GET, "/api/admin/advertisements", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    let ad = body
        .as_array()
        .expect("ads")
        .iter()
        .find(|ad| ad["name"] == json!("MGMT Banner"))
        .expect("created ad listed")
        .clone();
    let mut keys: Vec<&str> = ad.as_object().expect("ad").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "active",
            "description",
            "expires_at",
            "id",
            "image_url",
            "link",
            "name",
            "priority",
            "size",
            "starts_at",
            "status",
        ],
    );
    assert_eq!(ad["status"], json!("live"));
    assert_eq!(ad["size"], json!("sidebar"));
    let ad_id = ad["id"].as_i64().expect("id");

    let (status, _, _) = send(
        &app,
        Method::PATCH,
        &format!("/api/admin/advertisements/{ad_id}/toggle"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) =
        send(&app, Method::GET, "/api/admin/advertisements", Some(&admin), None).await;
    let toggled = body
        .as_array()
        .expect("ads")
        .iter()
        .find(|ad| ad["id"] == json!(ad_id))
        .expect("still listed")
        .clone();
    assert_eq!(toggled["status"], json!("inactive"));

    // A scheduled window derives its status; the sidebar hides it.
    let (status, _, _) = send(
        &app,
        Method::PUT,
        &format!("/api/admin/advertisements/{ad_id}"),
        Some(&admin),
        Some(json!({
            "name": "MGMT Banner",
            "image_url": "https://example.com/banner.png",
            "starts_at": "2099-01-01T00:00:00Z",
            "active": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) =
        send(&app, Method::GET, "/api/admin/advertisements", Some(&admin), None).await;
    let scheduled = body
        .as_array()
        .expect("ads")
        .iter()
        .find(|ad| ad["id"] == json!(ad_id))
        .expect("still listed")
        .clone();
    assert_eq!(scheduled["status"], json!("scheduled"));
    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", None, None).await;
    assert!(
        body["advertisements"]
            .as_array()
            .expect("ads")
            .iter()
            .all(|ad| ad["id"] != json!(ad_id)),
        "scheduled ads stay out of the rotation"
    );

    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/admin/advertisements/{ad_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) =
        send(&app, Method::GET, "/api/admin/advertisements", Some(&admin), None).await;
    assert!(
        body.as_array().expect("ads").iter().all(|ad| ad["id"] != json!(ad_id)),
        "deleted"
    );
}

#[tokio::test]
async fn gear_item_management_is_admin_gated_and_round_trips() {
    let pool = setup().await;
    let app = mutamarket::server::test_router().await;

    sqlx::query("delete from users where name in ('Gear Admin', 'Gear Peasant')")
        .execute(&pool)
        .await
        .expect("clean users");
    sqlx::query("delete from gear_items where name like 'GEARMGMT %'")
        .execute(&pool)
        .await
        .expect("clean gear");
    let admin_id: i64 = sqlx::query_scalar(
        "insert into users (name, is_admin) values ('Gear Admin', true) returning id",
    )
    .fetch_one(&pool)
    .await
    .expect("admin");
    let admin = create_session(&pool, admin_id, None).await.expect("session");
    let peasant_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Gear Peasant') returning id")
            .fetch_one(&pool)
            .await
            .expect("peasant");
    let peasant = create_session(&pool, peasant_id, None).await.expect("session");

    // Gating: guests 401, non-admins 403.
    let (status, _, _) = send(&app, Method::GET, "/api/admin/gear-items", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body, _) =
        send(&app, Method::GET, "/api/admin/gear-items", Some(&peasant), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("Forbidden."));

    // Validation mirrors the legacy rules: image required (as a URL in
    // the rewrite), the link required and a URL.
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/api/admin/gear-items",
        Some(&admin),
        Some(json!({ "name": "GEARMGMT Missing image" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["image_url"][0], json!("The image url field is required."));
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/api/admin/gear-items",
        Some(&admin),
        Some(json!({
            "name": "GEARMGMT Missing link",
            "image_url": "https://example.com/mouse.png",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["link"][0], json!("The link field is required."));
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/api/admin/gear-items",
        Some(&admin),
        Some(json!({
            "name": "GEARMGMT Bad link",
            "image_url": "https://example.com/mouse.png",
            "link": "not-a-url",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["link"][0], json!("The link field must be a valid URL."));

    // Create, list with the exact legacy key set, toggle, update, delete.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/api/admin/gear-items",
        Some(&admin),
        Some(json!({
            "name": "GEARMGMT Mouse",
            "image_url": "https://example.com/mouse.png",
            "link": "https://geni.us/mouse",
            "priority": 3,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body, _) =
        send(&app, Method::GET, "/api/admin/gear-items", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    let item = body
        .as_array()
        .expect("gear items")
        .iter()
        .find(|item| item["name"] == json!("GEARMGMT Mouse"))
        .expect("created gear item listed")
        .clone();
    let mut keys: Vec<&str> = item.as_object().expect("item").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["active", "description", "id", "image_url", "link", "name", "priority"],
    );
    assert_eq!(item["active"], json!(true));
    assert_eq!(item["priority"], json!(3));
    let item_id = item["id"].as_i64().expect("id");

    let (status, _, _) = send(
        &app,
        Method::PATCH,
        &format!("/api/admin/gear-items/{item_id}/toggle"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) = send(&app, Method::GET, "/api/admin/gear-items", Some(&admin), None).await;
    let toggled = body
        .as_array()
        .expect("gear items")
        .iter()
        .find(|item| item["id"] == json!(item_id))
        .expect("still listed")
        .clone();
    assert_eq!(toggled["active"], json!(false));

    // A toggled-off item leaves the sidebar rotation.
    let (_, body, _) = send(&app, Method::GET, "/api/sidebar", None, None).await;
    assert!(
        body["gear_items"]
            .as_array()
            .expect("gear")
            .iter()
            .all(|item| item["id"] != json!(item_id)),
        "inactive gear stays out of the rotation"
    );

    let (status, _, _) = send(
        &app,
        Method::PUT,
        &format!("/api/admin/gear-items/{item_id}"),
        Some(&admin),
        Some(json!({
            "name": "GEARMGMT Keyboard",
            "image_url": "https://example.com/keyboard.png",
            "link": "https://geni.us/keyboard",
            "description": "Clacky",
            "priority": 7,
            "active": true,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) = send(&app, Method::GET, "/api/admin/gear-items", Some(&admin), None).await;
    let updated = body
        .as_array()
        .expect("gear items")
        .iter()
        .find(|item| item["id"] == json!(item_id))
        .expect("still listed")
        .clone();
    assert_eq!(updated["name"], json!("GEARMGMT Keyboard"));
    assert_eq!(updated["description"], json!("Clacky"));
    assert_eq!(updated["priority"], json!(7));
    assert_eq!(updated["active"], json!(true));

    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/admin/gear-items/{item_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body, _) = send(&app, Method::GET, "/api/admin/gear-items", Some(&admin), None).await;
    assert!(
        body.as_array().expect("gear items").iter().all(|item| item["id"] != json!(item_id)),
        "deleted"
    );

    // Unknown ids answer the ported 404.
    let (status, body, _) = send(
        &app,
        Method::PATCH,
        &format!("/api/admin/gear-items/{item_id}/toggle"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], json!("Not found."));
}

#[tokio::test]
async fn launcher_store_campaigns_sync_into_the_rotation() {
    let pool = setup().await;
    sqlx::query("delete from advertisements where description = $1")
        .bind(mutamarket::advertisements::SYNC_MARKER)
        .execute(&pool)
        .await
        .expect("clean synced ads");
    let image_dir =
        std::env::temp_dir().join(format!("mutamarket-ads-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&image_dir);

    // A mock world: the site page linking a JS chunk that carries the
    // zone, the AdGlare feed, and the creative bytes.
    let feed = serde_json::json!({
        "response": {
            "success": 1,
            "campaigns": [
                {
                    "cID": "1", "crID": "111", "creative_type": "image",
                    "creative_data": {
                        "image_url": "https://creatives.example/store-a.png",
                        "landing_url": "https://store.eveonline.com/",
                        "click_url": "x", "target_window": "_blank"
                    },
                    "width": "445", "height": "500"
                },
                {
                    "cID": "2", "crID": "222", "creative_type": "image",
                    "creative_data": {
                        "image_url": "https://creatives.example/news.png",
                        "landing_url": "https://www.eveonline.com/news/view/something",
                        "click_url": "x", "target_window": "_blank"
                    },
                    "width": "445", "height": "500"
                }
            ]
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let feed_for_route = {
        let mut feed = feed.clone();
        // The store creative is served by the mock itself so the sync
        // can download it.
        feed["response"]["campaigns"][0]["creative_data"]["image_url"] =
            serde_json::json!(format!("{base}/creative/store-a.png"));
        feed
    };
    let app = axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async {
                axum::response::Html(
                    "<script src=\"/static/js/npm-x.1.js\"></script>\
                     <script src=\"/static/js/main.abc.chunk.js\"></script>",
                )
            }),
        )
        .route(
            "/static/js/main.abc.chunk.js",
            axum::routing::get(|| async { "var url = 'engine2.extccp.com/?424242';" }),
        )
        .route("/static/js/npm-x.1.js", axum::routing::get(|| async { "var nothing = 1;" }))
        .route(
            "/feed",
            axum::routing::get(move || {
                let feed = feed_for_route.clone();
                async move { axum::Json(feed) }
            }),
        )
        .route(
            "/creative/store-a.png",
            axum::routing::get(|| async { &b"fake png bytes"[..] }),
        );
    tokio::spawn(async move { axum::serve(listener, app).await.expect("mock world") });

    // Discovery finds the zone inside the main chunk.
    let discovered = mutamarket::advertisements::discover_feed_url(&format!("{base}/"))
        .await
        .expect("zone discovered");
    assert_eq!(discovered, "https://engine2.extccp.com/?424242&ag_custom_term=en");

    // First run mirrors only the store campaign and downloads its
    // creative; the rerun is a no-op.
    let feed_url = format!("{base}/feed");
    let report =
        mutamarket::advertisements::sync_launcher_store_ads(&pool, &feed_url, &image_dir)
            .await
            .expect("sync");
    assert_eq!(report.upserted, 1);
    assert_eq!(report.downloaded, 1);
    assert_eq!(report.removed, 0);
    assert!(image_dir.join("111.png").exists(), "creative stored locally");
    let report =
        mutamarket::advertisements::sync_launcher_store_ads(&pool, &feed_url, &image_dir)
            .await
            .expect("rerun");
    assert_eq!(report.upserted, 0, "idempotent rerun");
    assert_eq!(report.downloaded, 0, "existing files are kept");

    let (name, image_url, link, active): (String, String, String, bool) = sqlx::query_as(
        "select name, image_url, link, active from advertisements where description = $1",
    )
    .bind(mutamarket::advertisements::SYNC_MARKER)
    .fetch_one(&pool)
    .await
    .expect("synced row");
    assert_eq!(name, "EVE store promo 111");
    assert_eq!(image_url, "/img/ads/111.png", "served from our own copy");
    assert_eq!(link, mutamarket::advertisements::MARKEE_DRAGON_LINK);
    assert!(active);

    // A hand-made ad survives; a creative that left the feed loses its
    // row and its file.
    sqlx::query(
        "insert into advertisements (name, image_url, link, active, size)
         values ('Handmade', 'https://example.com/mine.png', null, true, 'sidebar')",
    )
    .execute(&pool)
    .await
    .expect("handmade ad");
    sqlx::query(
        "insert into advertisements (name, description, image_url, link, active, size)
         values ('EVE store promo 999', $1, '/img/ads/999.png', 'x', true, 'sidebar')",
    )
    .bind(mutamarket::advertisements::SYNC_MARKER)
    .execute(&pool)
    .await
    .expect("stale synced ad");
    std::fs::write(image_dir.join("999.png"), b"stale").expect("stale file");
    let report =
        mutamarket::advertisements::sync_launcher_store_ads(&pool, &feed_url, &image_dir)
            .await
            .expect("cleanup run");
    assert_eq!(report.removed, 1, "the departed creative is removed");
    assert!(!image_dir.join("999.png").exists(), "its file is removed too");
    let handmade: i64 =
        sqlx::query_scalar("select count(*) from advertisements where name = 'Handmade'")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(handmade, 1, "hand-made ads are never touched");
    let _ = std::fs::remove_dir_all(&image_dir);
}
