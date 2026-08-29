//! The omega calculator endpoint, the legacy `OmegaCalculatorController`:
//! only the env-driven store sale percentages; the stacking math is
//! client-side (frontend/src/lib/omega.ts).
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn omega_calculator_serves_the_env_driven_sales() {
    // This is the only test in this binary, so the process env is ours.
    // SAFETY: no other thread reads these variables concurrently.
    unsafe {
        std::env::set_var(mutamarket::server::omega::MARKEEDRAGON_SALE_ENV, "20");
        std::env::remove_var(mutamarket::server::omega::EVESTORE_SALE_ENV);
    }

    let app = mutamarket::server::test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/omega-calculator")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("application/json"),
        "json content type: {content_type}"
    );

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");

    // The exact legacy prop shape: {sales: {markeedragon, evestore}},
    // raw strings from the env, null when unset.
    let mut keys: Vec<&str> = body
        .as_object()
        .expect("payload")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["sales"]);
    let mut sales: Vec<&str> = body["sales"]
        .as_object()
        .expect("sales")
        .keys()
        .map(String::as_str)
        .collect();
    sales.sort_unstable();
    assert_eq!(sales, ["evestore", "markeedragon"]);
    assert_eq!(body["sales"]["markeedragon"], json!("20"));
    assert_eq!(body["sales"]["evestore"], serde_json::Value::Null);
}
