//! Route inventory tests mirroring the legacy Laravel application
//! (`routes/web.php` and `routes/api.php` in the PHP project).
//!
//! Each test pins down the status-level contract of a group of routes.
//! Feature-level behavior gets its own tests as each controller is ported;
//! a failing case here means the route has not been ported yet.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use tokio::sync::OnceCell;
use tower::ServiceExt;

async fn app() -> Router {
    static ROUTER: OnceCell<Router> = OnceCell::const_new();
    ROUTER
        .get_or_init(mutamarket::server::test_router)
        .await
        .clone()
}

async fn send(method: Method, path: &str) -> Response {
    app()
        .await
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible")
}

fn location(response: &Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn content_type(response: &Response) -> String {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

/// Runs `expectation` against every case and reports all failures at once,
/// so a single run shows the full remaining backlog for the group.
async fn check(
    cases: &[(Method, &str)],
    expected: &str,
    expectation: impl Fn(&Response) -> bool,
) {
    let mut failures = Vec::new();

    for (method, path) in cases {
        let response = send(method.clone(), path).await;
        if !expectation(&response) {
            failures.push(format!("  {method} {path} -> {}", response.status()));
        }
    }

    assert!(
        failures.is_empty(),
        "expected {expected} for {} route(s):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

fn redirects_to_login(response: &Response) -> bool {
    response.status().is_redirection() && location(response) == "/login"
}

/// The route is registered, even if its behavior is not implemented yet.
fn route_exists(response: &Response) -> bool {
    response.status() != StatusCode::NOT_FOUND
        && response.status() != StatusCode::METHOD_NOT_ALLOWED
}

async fn unknown_entities_return_not_found() {
    let pages = [
        (Method::GET, "/og/module/999999999"),
        (Method::GET, "/og/type/999999999"),
        (Method::GET, "/og/character/999999999"),
        (Method::GET, "/og/collection/999999999"),
    ];

    check(&pages, "404 Not Found", |response| {
        response.status() == StatusCode::NOT_FOUND
    })
    .await;
}

async fn guests_are_redirected_from_authenticated_actions() {
    let actions = [
        (Method::POST, "/personal/modules"),
        (Method::PUT, "/characters/1"),
        (Method::POST, "/public-assets"),
        (Method::DELETE, "/public-assets/1"),
        (Method::POST, "/estimate/1"),
        (Method::POST, "/settings"),
        (Method::PUT, "/settings"),
        (Method::POST, "/offers"),
        (Method::DELETE, "/offers/1"),
        (Method::POST, "/messages"),
        (Method::POST, "/collections"),
        (Method::POST, "/collections/modules"),
        (Method::PUT, "/collections/1"),
        (Method::DELETE, "/collections/1"),
        (Method::POST, "/collection-modules"),
        (Method::PUT, "/collection-modules/1"),
        (Method::DELETE, "/collection-modules/all"),
        (Method::DELETE, "/collection-modules/1"),
        (Method::POST, "/collection-locations"),
        (Method::PUT, "/collection-locations"),
        (Method::DELETE, "/collection-locations"),
        (Method::POST, "/location-collections"),
        (Method::POST, "/collections/1/auto-sync"),
        (Method::DELETE, "/collections/1/auto-sync"),
        (Method::POST, "/collections/1/auto-sync/locations"),
        (Method::DELETE, "/collections/1/auto-sync/locations/2"),
        (Method::POST, "/bookmarks"),
        (Method::PUT, "/bookmarks/1"),
        (Method::DELETE, "/bookmarks/1"),
        (Method::POST, "/ui/contract"),
        (Method::POST, "/personal/contracts"),
        (Method::POST, "/workbench/123456"),
        (Method::POST, "/workbench-modules"),
        (Method::PUT, "/workbench-modules/1"),
        (Method::DELETE, "/workbench-modules/all"),
        (Method::DELETE, "/workbench-modules/1"),
        (Method::POST, "/workbench-collections"),
        (Method::POST, "/logout"),
        (Method::DELETE, "/auth/character/1"),
        (Method::PUT, "/auth/character/1"),
        (Method::POST, "/module-pricing"),
        (Method::POST, "/notes"),
        (Method::POST, "/collection-notes"),
        (Method::PUT, "/raffle/1"),
        (Method::DELETE, "/raffle/1"),
        (Method::POST, "/blocked-users"),
        (Method::PUT, "/historic-contracts/1"),
        (Method::POST, "/raffles"),
        (Method::POST, "/advertisements"),
        (Method::POST, "/advertisements/1"),
        (Method::PATCH, "/advertisements/1/toggle"),
        (Method::DELETE, "/advertisements/1"),
        (Method::POST, "/gear-items"),
        (Method::POST, "/gear-items/1"),
        (Method::PATCH, "/gear-items/1/toggle"),
        (Method::DELETE, "/gear-items/1"),
        (Method::POST, "/moderator/contracts/1"),
        (Method::PUT, "/discord"),
        (Method::PUT, "/twitch"),
        (Method::PUT, "/patreon"),
    ];

    check(&actions, "redirect to /login", redirects_to_login).await;
}

async fn corporation_login_hops_through_the_eve_login() {
    // Like the legacy CorporationScopeController: an internal redirect to
    // /eve with the corporation assets scope.
    let response = send(Method::GET, "/eve/corporation").await;
    assert!(response.status().is_redirection());
    assert_eq!(location(&response), "/eve?scopes=esi-assets.read_corporation_assets.v1");
}

async fn oauth_flows_redirect_to_their_provider() {
    let flows = [
        (Method::GET, "/eve", "eveonline.com"),
        (Method::GET, "/eve/admin", "eveonline.com"),
        (Method::GET, "/twitch", "twitch.tv"),
        (Method::GET, "/discord", "discord.com"),
        (Method::GET, "/patreon", "patreon.com"),
    ];

    let mut failures = Vec::new();
    for (method, path, provider) in flows {
        let response = send(method, path).await;
        if !(response.status().is_redirection() && location(&response).contains(provider)) {
            failures.push(format!("  GET {path} -> {}", response.status()));
        }
    }

    assert!(
        failures.is_empty(),
        "expected redirect to the OAuth provider for {} route(s):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

async fn oauth_callbacks_are_registered() {
    let callbacks = [
        (Method::GET, "/eve/callback"),
        (Method::GET, "/twitch/callback"),
        (Method::GET, "/discord/callback"),
        (Method::GET, "/patreon/callback"),
    ];

    check(&callbacks, "a registered route", route_exists).await;
}

async fn public_submission_routes_are_registered() {
    // Module submission and display preferences are available to guests
    // in the legacy app.
    let routes = [
        (Method::POST, "/modules"),
        (Method::PUT, "/display"),
    ];

    check(&routes, "a registered route", route_exists).await;
}

async fn api_statistics_endpoints_return_json() {
    let endpoints = [
        (Method::GET, "/api/estimator-statistics"),
        (Method::GET, "/api/abyssal-type-statistics"),
    ];

    check(&endpoints, "200 OK with JSON", |response| {
        response.status() == StatusCode::OK
            && content_type(response).starts_with("application/json")
    })
    .await;
}

async fn api_module_index_requires_a_type() {
    // Mirrors the legacy behavior: the index rejects queries without a
    // valid `type/{id-or-slug}` segment.
    let requests = [
        (Method::GET, "/api/modules"),
        (Method::GET, "/api/modules/sort/price/asc"),
        (Method::GET, "/api/modules/type/not-a-real-type-slug"),
    ];

    check(&requests, "404 with JSON error", |response| {
        response.status() == StatusCode::NOT_FOUND
            && content_type(response).starts_with("application/json")
    })
    .await;
}

async fn api_unknown_module_returns_not_found() {
    let response = send(Method::GET, "/api/modules/hypnotic-web-999999999").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn api_module_submission_validates_empty_requests() {
    let response = send(Method::POST, "/api/modules").await;
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected POST /api/modules without payload to fail validation",
    );
}

/// The personal JSON endpoints answer guests with a 401 instead of the
/// page routes' login redirect: they only ever serve fetch() clients.
async fn personal_endpoints_require_auth() {
    let endpoints = [
        (Method::GET, "/api/personal/page"),
        (Method::GET, "/api/personal/modules"),
    ];

    check(&endpoints, "401 with JSON", |response| {
        response.status() == StatusCode::UNAUTHORIZED
            && content_type(response).starts_with("application/json")
    })
    .await;
}

/// The admin endpoints answer guests with a 401 (non-admin users get a
/// 403, pinned in tests/admin_scheduler.rs).
async fn admin_endpoints_require_auth() {
    let endpoints = [
        (Method::GET, "/api/admin/scheduler"),
        (Method::GET, "/api/admin/telemetry"),
        (Method::POST, "/api/admin/scheduler/stale-asset-imports/run"),
        (Method::PUT, "/api/admin/scheduler/stale-asset-imports"),
        (Method::PUT, "/api/historic-contracts/1"),
    ];

    check(&endpoints, "401 with JSON", |response| {
        response.status() == StatusCode::UNAUTHORIZED
            && content_type(response).starts_with("application/json")
    })
    .await;
}

/// The JSON endpoints backing the frontend pages (the former Leptos server
/// functions); the group grows as the endpoints land.
async fn page_data_endpoints_return_json() {
    let endpoints = [
        (Method::GET, "/api/nav-state"),
        (Method::GET, "/api/documentation"),
        (Method::GET, "/api/documentation/getting-started"),
        (Method::GET, "/api/module-cards"),
        (Method::GET, "/api/omega-calculator"),
        (Method::GET, "/api/module-stats"),
        (Method::GET, "/api/characters"),
        (Method::GET, "/api/collections"),
    ];

    check(&endpoints, "200 OK with JSON", |response| {
        response.status() == StatusCode::OK
            && content_type(response).starts_with("application/json")
    })
    .await;

    let not_found = [
        (Method::GET, "/api/documentation/no-such-page"),
        (Method::GET, "/api/module-page/unknown-999999999"),
        (Method::GET, "/api/module-page/unknown-999999999/similar"),
        (Method::GET, "/api/module-cards/type/not-a-real-type-slug"),
        (Method::GET, "/api/filter-panel/not-a-real-type-slug"),
        (Method::GET, "/api/characters/999999999"),
        (Method::GET, "/api/collections/999999999"),
    ];
    check(&not_found, "404 with JSON error", |response| {
        response.status() == StatusCode::NOT_FOUND
            && content_type(response).starts_with("application/json")
    })
    .await;
}

/// Anything not owned by the API answers a JSON 404; pages belong to the
/// SvelteKit frontend behind the shared proxy.
async fn fallback_returns_json_not_found() {
    let unowned = [
        (Method::GET, "/this-page-does-not-exist"),
        // Former server-rendered pages: the frontend owns them now.
        (Method::GET, "/"),
        (Method::GET, "/login"),
        (Method::GET, "/modules/damage-control"),
        // Not listed: paths whose non-GET methods stay registered (e.g.
        // POST /personal/modules) answer 405 to a GET; the proxy keeps
        // page GETs in the frontend either way.
    ];

    check(&unowned, "404 with JSON", |response| {
        response.status() == StatusCode::NOT_FOUND
            && content_type(response).starts_with("application/json")
    })
    .await;
}

/// The route-contract groups run sequentially on a single runtime and a
/// single shared router. Splitting them into parallel `#[tokio::test]`s
/// made 14 tokio runtimes share one connection pool, which exhausted it
/// (pool-acquire timeouts) and raced its shutdown ("Tokio context is being
/// shutdown") — surfacing as spurious 500s. One test, one runtime, one
/// pool, sequential requests: no contention.
#[tokio::test]
async fn route_contracts() {
    unknown_entities_return_not_found().await;
    guests_are_redirected_from_authenticated_actions().await;
    corporation_login_hops_through_the_eve_login().await;
    oauth_flows_redirect_to_their_provider().await;
    oauth_callbacks_are_registered().await;
    public_submission_routes_are_registered().await;
    api_statistics_endpoints_return_json().await;
    api_module_index_requires_a_type().await;
    api_unknown_module_returns_not_found().await;
    api_module_submission_validates_empty_requests().await;
    personal_endpoints_require_auth().await;
    admin_endpoints_require_auth().await;
    page_data_endpoints_return_json().await;
    fallback_returns_json_not_found().await;
}
