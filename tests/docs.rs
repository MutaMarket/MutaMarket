//! Behavior tests for the documentation pages: the vendored legacy docs
//! rendered through the full router, with the legacy index/404 behavior.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get_page(app: &Router, path: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn documentation_renders_the_legacy_pages() {
    let app = mutamarket::server::test_router().await;

    // The index shows the first page, like the legacy controller default.
    let (status, html) = get_page(&app, "/documentation").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Getting Started - MutaMarket"), "first-page title");
    assert!(html.contains("Documentation // Introduction"), "section label");
    assert!(html.contains("docs-prose"), "prose article present");
    assert!(html.contains("class=\"docs-anchor\""), "heading permalinks render");
    assert!(
        html.contains("https://github.com/MutaMarket/MutaMarket/edit/main/docs/01-getting-started.md"),
        "edit link points at the upstream file",
    );
    assert!(!html.contains("Previous"), "the first page has no previous link");
    assert!(html.contains("Next"), "the first page links to the next one");

    // A specific page renders its own heading and neighbours.
    let (status, html) = get_page(&app, "/documentation/premium").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Premium - MutaMarket"), "page title");
    assert!(html.contains("Documentation // Account"), "section label");
    assert!(html.contains("Previous"), "a middle page has a previous link");

    // The last page has no next link.
    let (status, html) = get_page(&app, "/documentation/legal").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Legal - MutaMarket"), "last-page title");
    assert!(html.contains("Documentation // General"), "default section label");
    assert!(html.contains("Previous"), "the last page has a previous link");
    assert!(!html.contains("Next \u{2192}"), "the last page has no next link");

    // /about redirects here and must keep rendering (routes.rs contract).
    let (status, _) = get_page(&app, "/documentation/about").await;
    assert_eq!(status, StatusCode::OK);

    // Every vendored page renders, and the sidebar lists all of them.
    let (_, index_html) = get_page(&app, "/documentation").await;
    for slug in [
        "getting-started",
        "browsing-the-market",
        "module-details",
        "appraisal",
        "rolling-guide",
        "selling-and-assets",
        "offers",
        "collections",
        "historic-sales",
        "workbench-and-tools",
        "premium",
        "donations-and-raffles",
        "support",
        "about",
        "legal",
    ] {
        assert!(
            index_html.contains(&format!("/documentation/{slug}")),
            "sidebar links {slug}",
        );

        let (status, _) = get_page(&app, &format!("/documentation/{slug}")).await;
        assert_eq!(status, StatusCode::OK, "{slug} renders");
    }
}

#[tokio::test]
async fn unknown_documentation_pages_are_404() {
    let app = mutamarket::server::test_router().await;

    let (status, _) = get_page(&app, "/documentation/no-such-page").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The legacy slug pattern is lowercase; other casings are unknown slugs.
    let (status, _) = get_page(&app, "/documentation/Legal").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
