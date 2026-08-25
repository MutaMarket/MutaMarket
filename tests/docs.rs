//! Behavior tests for the documentation pages: the vendored legacy docs
//! rendered through the full router, with the legacy index/404 behavior.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tokio::sync::OnceCell;
use tower::ServiceExt;

async fn app() -> Router {
    static ROUTER: OnceCell<Router> = OnceCell::const_new();
    ROUTER
        .get_or_init(mutamarket::server::test_router)
        .await
        .clone()
}

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

async fn documentation_renders_the_legacy_pages() {
    let app = app().await;

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

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> =
        value.as_object().expect("a JSON object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

async fn get_json(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
    let (status, body) = get_page(app, path).await;
    (status, serde_json::from_str(&body).expect("JSON body"))
}

/// The vendored pages in filename order, with their sidebar sections.
const LEGACY_PAGES: [(&str, &str); 15] = [
    ("getting-started", "Introduction"),
    ("browsing-the-market", "Modules"),
    ("module-details", "Modules"),
    ("appraisal", "Modules"),
    ("rolling-guide", "Modules"),
    ("selling-and-assets", "Trading"),
    ("offers", "Trading"),
    ("collections", "Trading"),
    ("historic-sales", "Trading"),
    ("workbench-and-tools", "Tools"),
    ("premium", "Account"),
    ("donations-and-raffles", "Account"),
    ("support", "General"),
    ("about", "General"),
    ("legal", "General"),
];

async fn api_documentation_serves_the_page_payload() {
    let app = app().await;

    // The index serves the first page, like the legacy controller default.
    let (status, body) = get_json(&app, "/api/documentation").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        ["edit_url", "html", "next", "previous", "section", "sections", "slug", "title"],
    );
    assert_eq!(body["slug"], "getting-started");
    assert_eq!(body["section"], "Introduction");
    assert_eq!(body["title"], "Getting Started");
    assert!(body["html"].as_str().expect("html").contains("docs-anchor"));
    assert_eq!(
        body["edit_url"],
        "https://github.com/MutaMarket/MutaMarket/edit/main/docs/01-getting-started.md",
    );
    assert_eq!(body["previous"], serde_json::Value::Null, "the first page has no previous");
    assert_eq!(sorted_keys(&body["next"]), ["slug", "title"]);
    assert_eq!(body["next"]["slug"], "browsing-the-market");

    // Sections group in first-seen order and cover every page in order.
    let sections = body["sections"].as_array().expect("sections");
    let mut expected_sections: Vec<&str> = Vec::new();
    for (_, section) in LEGACY_PAGES {
        if !expected_sections.contains(&section) {
            expected_sections.push(section);
        }
    }
    let section_titles: Vec<&str> =
        sections.iter().map(|section| section["title"].as_str().expect("title")).collect();
    assert_eq!(section_titles, expected_sections);
    for section in sections {
        assert_eq!(sorted_keys(section), ["pages", "title"]);
        for page in section["pages"].as_array().expect("pages") {
            assert_eq!(sorted_keys(page), ["slug", "title"]);
        }
    }
    let mut listed: Vec<String> = Vec::new();
    for (slug, section_title) in LEGACY_PAGES {
        let section = sections
            .iter()
            .find(|section| section["title"] == section_title)
            .expect("section listed");
        assert!(
            section["pages"].as_array().expect("pages").iter().any(|page| page["slug"] == slug),
            "{slug} listed under {section_title}",
        );
        listed.push(slug.to_owned());
    }
    assert_eq!(listed.len(), 15, "every vendored page is listed");

    // A middle page has both neighbours; a request by slug matches it.
    let (status, body) = get_json(&app, "/api/documentation/premium").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["slug"], "premium");
    assert_eq!(body["section"], "Account");
    assert_eq!(body["previous"]["slug"], "workbench-and-tools");
    assert_eq!(body["next"]["slug"], "donations-and-raffles");

    // The last page has no next link.
    let (status, body) = get_json(&app, "/api/documentation/legal").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["next"], serde_json::Value::Null);

    // Unknown slugs 404 with the exact message; the slug pattern is
    // lowercase, so other casings are unknown slugs.
    for path in ["/api/documentation/no-such-page", "/api/documentation/Legal"] {
        let (status, body) = get_json(&app, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(sorted_keys(&body), ["message"]);
        assert_eq!(body["message"], "This documentation page does not exist.");
    }
}

/// One test, one runtime, one shared router: parallel `#[tokio::test]`s
/// race the process-global SSR executor state across runtimes (a runtime
/// finishing while another still renders parks the latter forever), the
/// same failure mode the route contracts hit — see `tests/routes.rs`.
#[tokio::test]
async fn documentation_contracts() {
    documentation_renders_the_legacy_pages().await;
    api_documentation_serves_the_page_payload().await;
    unknown_documentation_pages_are_404().await;
}

async fn unknown_documentation_pages_are_404() {
    let app = app().await;

    let (status, _) = get_page(&app, "/documentation/no-such-page").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The legacy slug pattern is lowercase; other casings are unknown slugs.
    let (status, _) = get_page(&app, "/documentation/Legal").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
