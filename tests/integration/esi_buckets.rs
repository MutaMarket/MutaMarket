//! Proves the per-bucket adaptive limiter (`src/esi/buckets.rs`) actually
//! self-paces the client: a response whose `X-Ratelimit-Remaining` is at
//! or below the safety margin makes the *next* request to the same
//! (group, subject) wait out the window before it fires, instead of
//! firing immediately and risking a 429.
//!
//! Needs no database (the client alone is under test), unlike most other
//! suites in this binary.

use crate::common::EnvGuard;

use axum::Router;
use axum::routing::get;
use mutamarket::esi::EsiClient;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Any real region/type id works: the mock ignores the path params and
/// answers every request the same way.
const FORGE_REGION_ID: i64 = 10_000_002;
const PLEX_TYPE_ID: i64 = 44_992;

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

#[tokio::test]
async fn a_low_remaining_bucket_paces_the_next_request_to_the_same_bucket() {
    let request_count = Arc::new(AtomicUsize::new(0));
    let counter = request_count.clone();
    let router = Router::new().route(
        "/latest/markets/{region_id}/history/",
        get(move || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                // A 1-second window with only 3 tokens left of 10 — below
                // the margin set through ESI_BUCKET_MARGIN below.
                (
                    [
                        ("x-ratelimit-group", "market-history-test"),
                        ("x-ratelimit-limit", "10/1s"),
                        ("x-ratelimit-remaining", "3"),
                        ("x-ratelimit-used", "7"),
                    ],
                    axum::Json(serde_json::json!([])),
                )
            }
        }),
    );
    let base = start_mock(router).await;

    // Guarded: EsiClient::from_env() reads all three, and a leaked value
    // would repoint or re-pace every later suite's ESI client.
    let _esi_base = EnvGuard::capture("ESI_BASE_URL");
    let _max_rps = EnvGuard::capture("ESI_MAX_RPS");
    let _margin = EnvGuard::capture("ESI_BUCKET_MARGIN");
    // SAFETY: the suites run single-threaded (see tests/integration/main.rs),
    // so no other thread reads these variables concurrently.
    unsafe {
        std::env::set_var("ESI_BASE_URL", &base);
        // Isolates the bucket door: any wait observed below must come
        // from it, not from the flat global request-rate cap.
        std::env::set_var("ESI_MAX_RPS", "0");
        std::env::set_var("ESI_BUCKET_MARGIN", "5");
    }

    let esi = EsiClient::from_env();

    // First call: nothing is learned yet, so it fires immediately and
    // teaches the limiter the group and its (low) remaining budget.
    let started = Instant::now();
    esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID)
        .await
        .expect("first call");
    assert!(started.elapsed() < Duration::from_millis(200));
    assert_eq!(request_count.load(Ordering::SeqCst), 1);

    // Second call to the same (group, Public) bucket: remaining (3) is at
    // or below the margin (5), so the door waits out the ~1s window.
    let started = Instant::now();
    esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID)
        .await
        .expect("second call");
    let elapsed = started.elapsed();

    assert_eq!(request_count.load(Ordering::SeqCst), 2);
    assert!(
        elapsed >= Duration::from_millis(900),
        "second call should have waited out the window, elapsed {elapsed:?}"
    );
}

#[tokio::test]
async fn an_unrelated_bucket_is_not_paced_by_a_different_groups_low_budget() {
    let router = Router::new()
        .route(
            "/latest/markets/{region_id}/history/",
            get(|| async {
                (
                    [
                        ("x-ratelimit-group", "market-history-isolated"),
                        ("x-ratelimit-limit", "10/1s"),
                        ("x-ratelimit-remaining", "0"),
                    ],
                    axum::Json(serde_json::json!([])),
                )
            }),
        )
        .route(
            "/latest/alliances/",
            get(|| async {
                (
                    [
                        ("x-ratelimit-group", "alliances-isolated"),
                        ("x-ratelimit-limit", "10/1s"),
                        ("x-ratelimit-remaining", "9"),
                    ],
                    axum::Json(serde_json::json!([])),
                )
            }),
        );
    let base = start_mock(router).await;

    let _esi_base = EnvGuard::capture("ESI_BASE_URL");
    let _max_rps = EnvGuard::capture("ESI_MAX_RPS");
    let _margin = EnvGuard::capture("ESI_BUCKET_MARGIN");
    // SAFETY: see the suite-level note above.
    unsafe {
        std::env::set_var("ESI_BASE_URL", &base);
        std::env::set_var("ESI_MAX_RPS", "0");
        std::env::set_var("ESI_BUCKET_MARGIN", "5");
    }

    let esi = EsiClient::from_env();

    // Exhausts market-history-isolated's budget for the Public subject.
    esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID)
        .await
        .expect("market history call");

    // A different group (also Public) is unaffected: no wait.
    let started = Instant::now();
    esi.alliance_ids().await.expect("alliances call");
    assert!(started.elapsed() < Duration::from_millis(200));
}
