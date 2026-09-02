//! Server-side translation of API error sentences (legacy `__()` with
//! `lang/{de,zh}.json`): the locale cookie first, then Accept-Language,
//! else English; unknown sentences pass through untouched.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn add_module_failure(headers: &[(&str, &str)]) -> (StatusCode, String) {
    let app = mutamarket::server::test_router().await;
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/modules")
        .header(header::CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .oneshot(
            request
                .body(Body::from(r#"{"message":"no module link in here"}"#))
                .expect("request"),
        )
        .await
        .expect("infallible");
    let status = response.status();
    let body = response.into_body().collect().await.expect("body").to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
    (status, json["message"].as_str().unwrap_or_default().to_owned())
}

#[tokio::test]
async fn error_sentences_follow_the_locale_cookie_then_accept_language() {
    let (status, english) = add_module_failure(&[]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(english, "Failed to add module!");

    let (_, german) = add_module_failure(&[("cookie", "locale=de")]).await;
    assert_eq!(german, "Modul konnte nicht hinzugefügt werden!");

    // The cookie outranks the header; the header alone still counts.
    let (_, cookie_wins) =
        add_module_failure(&[("cookie", "locale=en"), ("accept-language", "de")]).await;
    assert_eq!(cookie_wins, "Failed to add module!");
    let (_, chinese) = add_module_failure(&[("accept-language", "zh-CN,zh;q=0.9,en;q=0.5")]).await;
    assert_ne!(chinese, "Failed to add module!");
    assert!(!chinese.is_ascii(), "translated to Chinese: {chinese}");
}
