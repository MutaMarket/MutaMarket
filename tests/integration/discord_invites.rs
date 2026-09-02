//! The Discord invite member counts (the legacy DiscordWidgetService +
//! DiscordInvites shared prop): the scheduler-job refresh against a
//! mock Discord API, its persistence in app_settings, and the counts
//! on the /api/sidebar payload.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::collections::HashMap;

use crate::common::EnvGuard;

use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::discord_invites::{
    ABYSSAL_TRADING_INVITE_ENV, DISCORD_INVITE_ENV, ECTRADE_INVITE_ENV, MEMBER_COUNT_KEY_PREFIX,
    refresh_member_counts,
};
use serde_json::json;
use tower::ServiceExt;

/// The live invite's count served by the mock API.
const LIVE_MEMBER_COUNT: i64 = 12_543;

async fn start_mock_discord() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let app = axum::Router::new().route(
        "/invites/{code}",
        axum::routing::get(
            |Path(code): Path<String>, Query(query): Query<HashMap<String, String>>| async move {
                assert_eq!(
                    query.get("with_counts").map(String::as_str),
                    Some("true"),
                    "the legacy service always asks for counts"
                );
                match code.as_str() {
                    "abyss123" => axum::Json(json!({
                        "code": "abyss123",
                        "approximate_member_count": LIVE_MEMBER_COUNT,
                        "approximate_presence_count": 900,
                    }))
                    .into_response(),
                    _ => StatusCode::NOT_FOUND.into_response(),
                }
            },
        ),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.expect("mock discord") });
    base
}

#[tokio::test]
async fn member_counts_refresh_and_reach_the_sidebar_payload() {
    let _abyssal = EnvGuard::capture(ABYSSAL_TRADING_INVITE_ENV);
    let _discord = EnvGuard::capture(DISCORD_INVITE_ENV);
    let _ectrade = EnvGuard::capture(ECTRADE_INVITE_ENV);
    // SAFETY: the suite runs single-threaded (see tests/integration/main.rs),
    // so no other thread reads these variables concurrently.
    unsafe {
        std::env::set_var(ABYSSAL_TRADING_INVITE_ENV, "https://discord.gg/abyss123");
        std::env::set_var(DISCORD_INVITE_ENV, "https://discord.gg/gone999");
        std::env::remove_var(ECTRADE_INVITE_ENV);
    }

    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    sqlx::query("delete from app_settings where key like $1")
        .bind(format!("{MEMBER_COUNT_KEY_PREFIX}%"))
        .execute(&pool)
        .await
        .expect("clean counts");
    // A stale count for the invite the API no longer knows: the refresh
    // must clear it, like the legacy caching the null fetch.
    mutamarket::app_settings::set(&pool, &format!("{MEMBER_COUNT_KEY_PREFIX}gone999"), "777")
        .await
        .expect("stale count");

    let mock = start_mock_discord().await;
    let invites = [
        "https://discord.gg/gone999".to_owned(),
        "https://discord.gg/abyss123".to_owned(),
    ];
    let stats = refresh_member_counts(&pool, &mock, &invites)
        .await
        .expect("refresh");
    assert_eq!(stats.stored, 1);
    assert_eq!(stats.unavailable, 1);

    let live = mutamarket::app_settings::get(&pool, &format!("{MEMBER_COUNT_KEY_PREFIX}abyss123"))
        .await
        .expect("read count");
    assert_eq!(live.as_deref(), Some("12543"));
    let gone = mutamarket::app_settings::get(&pool, &format!("{MEMBER_COUNT_KEY_PREFIX}gone999"))
        .await
        .expect("read stale");
    assert_eq!(gone, None, "a failed fetch clears the stored count");

    // The payload: the legacy shared-prop shape, counts from the store.
    let app = mutamarket::server::test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/sidebar")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");

    let invites = body["discord_invites"].as_array().expect("invites");
    assert_eq!(invites.len(), 3);
    for invite in invites {
        let mut keys: Vec<&str> = invite
            .as_object()
            .expect("invite")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["image", "member_count", "name", "url"]);
    }
    assert_eq!(
        invites[0],
        json!({
            "name": "Abyssal Trading",
            "url": "https://discord.gg/abyss123",
            "image": "/img/at.webp",
            "member_count": LIVE_MEMBER_COUNT,
        })
    );
    assert_eq!(
        invites[1],
        json!({
            "name": "MutaMarket",
            "url": "https://discord.gg/gone999",
            "image": null,
            "member_count": null,
        })
    );
    assert_eq!(
        invites[2],
        json!({
            "name": "EC Trade",
            "url": null,
            "image": "/img/ectrade.png",
            "member_count": null,
        })
    );
}
