//! The cross-site mutation lock: a non-GET request the browser marks as
//! cross-site, or whose Origin names another host, is refused before any
//! handler runs; same-origin and header-less requests pass through and
//! reads are never blocked. Also the stale active character: a session's
//! pick only counts while the account still owns the character.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use tower::ServiceExt;

async fn status(app: &Router, method: &str, uri: &str, headers: &[(&str, &str)]) -> StatusCode {
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("infallible")
        .status()
}

#[tokio::test]
async fn cross_site_mutations_are_refused() {
    let app = mutamarket::server::test_router().await;

    let plain = status(&app, "POST", "/logout", &[]).await;
    assert_ne!(
        plain,
        StatusCode::FORBIDDEN,
        "no browser headers: not cross-site"
    );

    assert_eq!(
        status(&app, "POST", "/logout", &[("sec-fetch-site", "cross-site")]).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            "POST",
            "/logout",
            &[
                ("origin", "https://evil.example"),
                ("host", "mutamarket.com")
            ],
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            "DELETE",
            "/workbench-modules/all",
            &[("origin", "null"), ("host", "mutamarket.com")],
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(
            &app,
            "POST",
            "/logout",
            &[
                ("origin", "https://mutamarket.com"),
                ("host", "mutamarket.com"),
                ("sec-fetch-site", "same-origin"),
            ],
        )
        .await,
        plain,
        "the site's own origin passes through"
    );
    assert_eq!(
        status(
            &app,
            "GET",
            "/api/nav-state",
            &[
                ("sec-fetch-site", "cross-site"),
                ("origin", "https://evil.example")
            ],
        )
        .await,
        StatusCode::OK,
        "reads are never blocked"
    );
}

const PILOT_KEPT: i64 = 990_700_001;
const PILOT_MOVED: i64 = 990_700_002;

#[tokio::test]
async fn a_stale_active_character_falls_back_to_an_owned_one() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let app = mutamarket::server::test_router().await;

    for name in ["Stale Pick Owner", "Stale Pick Buyer"] {
        sqlx::query("delete from users where name = $1")
            .bind(name)
            .execute(&pool)
            .await
            .expect("clean user");
    }
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![PILOT_KEPT, PILOT_MOVED])
        .execute(&pool)
        .await
        .expect("clean characters");
    let owner: i64 =
        sqlx::query_scalar("insert into users (name) values ('Stale Pick Owner') returning id")
            .fetch_one(&pool)
            .await
            .expect("owner");
    let buyer: i64 =
        sqlx::query_scalar("insert into users (name) values ('Stale Pick Buyer') returning id")
            .fetch_one(&pool)
            .await
            .expect("buyer");
    for (id, name) in [(PILOT_KEPT, "Kept Pilot"), (PILOT_MOVED, "Moved Pilot")] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(owner)
            .execute(&pool)
            .await
            .expect("character");
    }
    let session = mutamarket::auth::session::create_session(&pool, owner, Some(PILOT_MOVED))
        .await
        .expect("session");

    // The character changes hands (an EVE transfer, then the buyer logs
    // it in) while the owner's session still names it.
    sqlx::query("update characters set user_id = $1 where id = $2")
        .bind(buyer)
        .bind(PILOT_MOVED)
        .execute(&pool)
        .await
        .expect("transfer");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/sell/page")
                .header(header::COOKIE, format!("mm_session={session}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let page: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(
        page["character_id"].as_i64(),
        Some(PILOT_KEPT),
        "the session's stale pick yields to a character the account owns"
    );

    sqlx::query("delete from users where id = any($1)")
        .bind(vec![owner, buyer])
        .execute(&pool)
        .await
        .ok();
}
