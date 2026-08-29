//! Route test for /sitemap.xml: served by Axum with the exact URL set
//! the legacy spatie crawler produced (see src/server/sitemap.rs).
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn sitemap_serves_the_legacy_url_set_as_xml() {
    let app = mutamarket::server::test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/sitemap.xml")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/xml"),
    );

    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body = std::str::from_utf8(&body).expect("utf-8 xml");

    assert!(body.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
    assert!(
        body.contains(
            "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
             xmlns:xhtml=\"http://www.w3.org/1999/xhtml\" \
             xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\" \
             xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\" \
             xmlns:news=\"http://www.google.com/schemas/sitemap-news/0.9\">",
        ),
        "the spatie urlset root with its namespaces",
    );
    assert!(body.trim_end().ends_with("</urlset>"));

    // The exact <loc> set, in the legacy generator's output order.
    let locs: Vec<&str> = body
        .split("<loc>")
        .skip(1)
        .map(|part| part.split("</loc>").next().expect("closed loc"))
        .collect();
    let expected: Vec<String> = mutamarket::server::sitemap::SITEMAP_PATHS
        .iter()
        .map(|path| format!("https://mutamarket.com{path}"))
        .collect();
    assert_eq!(locs, expected);
    assert_eq!(locs.len(), 38, "the full legacy crawl output");
    for page in [
        "https://mutamarket.com/",
        "https://mutamarket.com/modules",
        "https://mutamarket.com/characters",
        "https://mutamarket.com/collections",
        "https://mutamarket.com/documentation/getting-started",
        "https://mutamarket.com/statistics",
        "https://mutamarket.com/all-modules/sort/value/desc",
    ] {
        assert!(locs.contains(&page), "{page} is listed");
    }
}
