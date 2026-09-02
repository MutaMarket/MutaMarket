//! Behavior tests for `PUT /display`: the guest-accessible display
//! preference endpoint persisting the three legacy cookies.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn put_display(app: &Router, body: &str) -> (StatusCode, Vec<String>, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/display")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_owned()))
                .expect("valid request"),
        )
        .await
        .expect("infallible");

    let status = response.status();
    let cookies: Vec<String> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_owned)
        .collect();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();

    (
        status,
        cookies,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

#[tokio::test]
async fn display_settings_persist_as_cookies() {
    let app = mutamarket::server::test_router().await;

    // Valid settings answer 204 with the three year-long cookies
    // (divergence from the legacy redirect-back, see server::display).
    let (status, cookies, body) = put_display(
        &app,
        r#"{"display":"grid","attribute_bar_mode":"type","show_attribute_scores":true}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "a 204 carries no body");
    assert_eq!(
        cookies,
        [
            "display=grid; Path=/; SameSite=Lax; Max-Age=31536000",
            "attribute_bar_mode=type; Path=/; SameSite=Lax; Max-Age=31536000",
            "show_attribute_scores=1; Path=/; SameSite=Lax; Max-Age=31536000",
        ],
    );

    // Laravel's boolean validation accepts "0"/"1" strings too.
    let (status, cookies, _) = put_display(
        &app,
        r#"{"display":"table","attribute_bar_mode":"none","show_attribute_scores":"0"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(
        cookies
            .iter()
            .any(|cookie| cookie.starts_with("show_attribute_scores=0;"))
    );

    // Invalid or missing values answer the exact legacy 422.
    for invalid in [
        r#"{"attribute_bar_mode":"sideways"}"#,
        r#"{"display":"grid","attribute_bar_mode":"type","show_attribute_scores":"maybe"}"#,
        r#"{}"#,
        "",
    ] {
        let (status, cookies, body) = put_display(&app, invalid).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "payload: {invalid}"
        );
        assert!(cookies.is_empty(), "no cookies on validation failure");
        let error: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(
            error["message"],
            serde_json::json!("The given data was invalid.")
        );
    }
}
