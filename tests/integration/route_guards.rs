//! The authorization matrix of every registered route. The routes are
//! read off `src/server/mod.rs` itself, so a route added without a row
//! here fails, and each row pins what a guest, a signed-in user and an
//! admin get from the guard: login redirect, JSON 401, 403, or a plain
//! answer. Ownership rules within a class (my offer, my collection) are
//! pinned by the feature suites; this suite guarantees no route is
//! unclassified.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use mutamarket::auth::session::create_session;
use mutamarket::db;
use tower::ServiceExt;

/// What the guard in front of a route lets through.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Guard {
    /// Anyone: the answer is never a login redirect or a 401/403.
    Public,
    /// Page-backed action: guests are redirected to `/login`.
    Login,
    /// Fetch-only endpoint: guests get a JSON 401.
    Api,
    /// Admin console API: guests 401, everyone else 403.
    Admin,
    /// Admin page action: guests are redirected to `/login`, everyone
    /// else gets the legacy 403.
    AdminPage,
    /// Premium feature: guests 401, plain users 403.
    Premium,
    /// Starts an OAuth flow: a redirect for everyone.
    Provider,
    /// The WebSocket upgrade: a plain request is a bad handshake for a
    /// guest, never a session leak.
    Socket,
}

use Guard::*;

const PLEB_CHARACTER: i64 = 990_800_001;

const ROUTES: &[(&str, &str, Guard)] = &[
    ("POST", "/modules", Public),
    ("PUT", "/display", Public),
    ("GET", "/ws", Socket),
    ("GET", "/og/module/{module}", Public),
    ("GET", "/og/type/{type}", Public),
    ("GET", "/og/character/{character}", Public),
    ("GET", "/og/collection/{collection}", Public),
    ("GET", "/sitemap.xml", Public),
    ("GET", "/eve", Provider),
    ("GET", "/eve/corporation", Provider),
    ("GET", "/eve/admin", Provider),
    ("GET", "/eve/callback", Provider),
    ("GET", "/twitch", Provider),
    ("PUT", "/twitch", Login),
    ("GET", "/twitch/callback", Provider),
    ("GET", "/discord", Provider),
    ("PUT", "/discord", Login),
    ("GET", "/discord/callback", Provider),
    ("GET", "/patreon", Provider),
    ("PUT", "/patreon", Login),
    ("GET", "/patreon/callback", Provider),
    ("POST", "/personal/modules", Login),
    ("PUT", "/characters/{character}", Login),
    ("PUT", "/characters/{character}/scope-warnings", Login),
    ("POST", "/public-assets", Login),
    ("DELETE", "/public-assets/{asset}", Login),
    ("POST", "/estimate/{module}", Login),
    ("PUT", "/settings", Login),
    ("PUT", "/settings/accent", Login),
    ("POST", "/premium/gift", Login),
    ("POST", "/offers", Login),
    ("DELETE", "/offers/{offer}", Login),
    ("POST", "/messages", Login),
    ("POST", "/collections", Login),
    ("POST", "/collections/modules", Login),
    ("PUT", "/collections/{collection}", Login),
    ("DELETE", "/collections/{collection}", Login),
    ("POST", "/collection-modules", Login),
    ("DELETE", "/collection-modules/all", Login),
    ("PUT", "/collection-modules/{collection_module}", Login),
    ("DELETE", "/collection-modules/{collection_module}", Login),
    ("POST", "/collection-locations", Login),
    ("PUT", "/collection-locations", Login),
    ("DELETE", "/collection-locations", Login),
    ("POST", "/location-collections", Login),
    ("POST", "/collections/{collection}/auto-sync", Login),
    ("DELETE", "/collections/{collection}/auto-sync", Login),
    (
        "POST",
        "/collections/{collection}/auto-sync/locations",
        Login,
    ),
    (
        "DELETE",
        "/collections/{collection}/auto-sync/locations/{asset}",
        Login,
    ),
    ("POST", "/bookmarks", Login),
    ("PUT", "/bookmarks/{bookmark}", Login),
    ("DELETE", "/bookmarks/{bookmark}", Login),
    ("POST", "/ui/contract", Login),
    ("POST", "/personal/contracts", Login),
    ("POST", "/workbench/{*modules}", Login),
    ("POST", "/workbench-modules", Login),
    ("DELETE", "/workbench-modules/all", Login),
    ("PUT", "/workbench-modules/{workbench_module}", Login),
    ("DELETE", "/workbench-modules/{workbench_module}", Login),
    ("POST", "/workbench-collections", Login),
    ("POST", "/logout", Login),
    ("PUT", "/auth/character/{character}", Login),
    ("DELETE", "/auth/character/{character}", Login),
    ("POST", "/module-pricing", Login),
    ("POST", "/notes", Login),
    ("POST", "/collection-notes", Login),
    ("PUT", "/raffle/{raffle_item}", Login),
    ("DELETE", "/raffle/{raffle_item}", Login),
    ("POST", "/blocked-users", Login),
    ("DELETE", "/blocked-users/{user}", Login),
    ("POST", "/raffles", AdminPage),
    // Any signed-in user may review, like the legacy route (no admin
    // middleware there).
    ("POST", "/moderator/contracts/{historic_contract}", Login),
    ("GET", "/api/health", Public),
    ("GET", "/api/modules", Public),
    ("POST", "/api/modules", Public),
    ("GET", "/api/modules/{*query}", Public),
    ("GET", "/api/openapi.json", Public),
    ("GET", "/api/estimator-statistics", Public),
    ("GET", "/api/abyssal-type-statistics", Public),
    ("GET", "/api/nav-state", Public),
    ("GET", "/api/documentation", Public),
    ("GET", "/api/documentation/{page}", Public),
    ("GET", "/api/module-page/{module}", Public),
    // Empty for non-premium viewers rather than refused, like the legacy
    // deferred prop.
    ("GET", "/api/module-page/{module}/similar", Public),
    ("GET", "/api/module-cards", Public),
    ("GET", "/api/module-cards/{*query}", Public),
    ("GET", "/api/premium/page", Public),
    ("GET", "/api/historic-sales-cards", Premium),
    ("GET", "/api/historic-sales-cards/{*query}", Premium),
    ("GET", "/api/module-stats", Public),
    ("GET", "/api/filter-panel/{type}", Public),
    ("GET", "/api/characters", Public),
    ("GET", "/api/characters/{character}", Public),
    ("GET", "/api/collections", Public),
    ("GET", "/api/collections/{collection}", Public),
    ("GET", "/api/statistics/overview", Public),
    ("GET", "/api/statistics/top", Public),
    ("GET", "/api/statistics/top/{*query}", Public),
    ("GET", "/api/personal/stats", Api),
    ("GET", "/api/settings", Api),
    ("GET", "/api/locations", Api),
    ("GET", "/api/locations/{location}", Api),
    ("GET", "/api/locations/{location}/{*query}", Api),
    ("GET", "/api/personal/page", Api),
    ("GET", "/api/personal/contracts", Api),
    ("GET", "/api/moderator/contracts", Public),
    ("GET", "/api/moderator/contracts/{*query}", Public),
    ("GET", "/api/personal/modules", Api),
    ("GET", "/api/calculator", Public),
    ("GET", "/api/calculator/{*query}", Public),
    ("GET", "/api/collections/module/{module}", Api),
    ("GET", "/api/sidebar", Public),
    ("GET", "/api/workbench", Api),
    ("GET", "/api/workbench-page/{*modules}", Public),
    ("GET", "/api/offers", Api),
    ("GET", "/api/offers/sent", Api),
    ("GET", "/api/offers/{offer}", Api),
    ("GET", "/api/sell/page", Api),
    ("GET", "/api/sell/modules", Api),
    ("GET", "/api/sell/locations", Api),
    ("GET", "/api/admin/advertisements", Admin),
    ("POST", "/api/admin/advertisements", Admin),
    ("PUT", "/api/admin/advertisements/{advertisement}", Admin),
    ("DELETE", "/api/admin/advertisements/{advertisement}", Admin),
    (
        "PATCH",
        "/api/admin/advertisements/{advertisement}/toggle",
        Admin,
    ),
    ("GET", "/api/admin/gear-items", Admin),
    ("POST", "/api/admin/gear-items", Admin),
    ("PUT", "/api/admin/gear-items/{gear_item}", Admin),
    ("DELETE", "/api/admin/gear-items/{gear_item}", Admin),
    ("PATCH", "/api/admin/gear-items/{gear_item}/toggle", Admin),
    ("GET", "/api/admin/raffles", Admin),
    ("GET", "/api/admin/live", Admin),
    ("GET", "/api/admin/activity", Admin),
    ("GET", "/api/admin/esi-failures", Admin),
    ("GET", "/api/admin/esi-failures/{failure}", Admin),
    ("GET", "/api/admin/scheduler", Admin),
    ("GET", "/api/admin/system", Admin),
    ("GET", "/api/admin/metrics", Admin),
    ("GET", "/api/admin/telemetry", Admin),
    ("POST", "/api/admin/scheduler/{job}/run", Admin),
    ("PUT", "/api/admin/scheduler/{job}", Admin),
    ("PUT", "/api/historic-contracts/{id}", Admin),
    ("GET", "/api/admin/service-character", Admin),
];

/// Every `(method, path)` the router registers, read off its source: the
/// string literal after each `.route(` and the method builders inside
/// the balanced parentheses that follow, prefixed by `/api` inside
/// `api_router`.
fn registered_routes() -> Vec<(String, String)> {
    let source = include_str!("../../src/server/mod.rs");
    let functions: Vec<(usize, &str)> = source
        .match_indices("fn ")
        .filter_map(|(at, _)| {
            let name_end = source[at + 3..].find('(')?;
            Some((at, &source[at + 3..at + 3 + name_end]))
        })
        .collect();
    let enclosing = |at: usize| {
        functions
            .iter()
            .filter(|(start, _)| *start < at)
            .map(|(_, name)| *name)
            .next_back()
    };

    let mut routes = Vec::new();
    let mut cursor = 0;
    while let Some(found) = source[cursor..].find(".route(") {
        let at = cursor + found;
        let literal_start = at + source[at..].find('"').expect("route path") + 1;
        let literal_end = literal_start + source[literal_start..].find('"').expect("path end");
        let path = &source[literal_start..literal_end];

        let mut depth = 1;
        let mut end = at + ".route(".len();
        while depth > 0 {
            match source.as_bytes()[end] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            end += 1;
        }
        let builders = &source[literal_end..end];
        let prefix = if enclosing(at) == Some("api_router") {
            "/api"
        } else {
            ""
        };
        for method in ["get", "post", "put", "delete", "patch"] {
            let mut from = 0;
            while let Some(hit) = builders[from..].find(&format!("{method}(")) {
                let position = from + hit;
                let boundary = position == 0
                    || !builders.as_bytes()[position - 1].is_ascii_alphanumeric()
                        && builders.as_bytes()[position - 1] != b'_';
                if boundary {
                    routes.push((method.to_uppercase(), format!("{prefix}{path}")));
                }
                from = position + method.len();
            }
        }
        cursor = end;
    }
    routes
}

/// A concrete request path for a route pattern; the guards answer before
/// any placeholder is resolved, so the values only need to parse.
fn concrete(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let end = rest[start..].find('}').expect("placeholder end") + start;
        let name = rest[start + 1..end].trim_start_matches('*');
        out.push_str(match name {
            "query" => "type/49738",
            "page" => "getting-started",
            "job" => "stale-asset-imports",
            "character" | "collection" | "location" => "x-1",
            _ => "1",
        });
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
) -> (StatusCode, String) {
    let mut request = Request::builder()
        .method(Method::from_bytes(method.as_bytes()).expect("method"))
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = session {
        request = request.header(header::COOKIE, format!("mm_session={token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from("{}")).expect("request"))
        .await
        .expect("infallible");
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    (response.status(), location)
}

#[tokio::test]
async fn every_route_has_a_pinned_guard() {
    let registered = registered_routes();
    assert!(
        registered.len() > 100,
        "the router parse found {} routes",
        registered.len()
    );

    let unclassified: Vec<String> = registered
        .iter()
        .filter(|(method, path)| !ROUTES.iter().any(|(m, p, _)| m == method && p == path))
        .map(|(method, path)| format!("{method} {path}"))
        .collect();
    assert!(
        unclassified.is_empty(),
        "routes without a guard row: {unclassified:?}"
    );
    let stale: Vec<String> = ROUTES
        .iter()
        .filter(|(m, p, _)| {
            !registered
                .iter()
                .any(|(method, path)| m == method && p == path)
        })
        .map(|(m, p, _)| format!("{m} {p}"))
        .collect();
    assert!(stale.is_empty(), "guard rows no route registers: {stale:?}");

    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let app = mutamarket::server::test_router().await;

    // A plain, non-premium account with no characters: enough to be a
    // session, never enough to pass an admin or premium gate.
    sqlx::query("delete from users where name = 'Route Guard Pleb'")
        .execute(&pool)
        .await
        .expect("clean user");
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Route Guard Pleb') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    // Every real account has at least one character; the handlers that
    // act as "the active character" rely on it.
    // The probe creates rows as the character (collections, workbench
    // conversions); they go before the character does.
    sqlx::query("delete from collections where character_id = $1")
        .bind(PLEB_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean collections");
    sqlx::query("delete from characters where id = $1")
        .bind(PLEB_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean character");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Route Guard Pilot', $2)")
        .bind(PLEB_CHARACTER)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("character");
    let user = create_session(&pool, user_id, Some(PLEB_CHARACTER))
        .await
        .expect("session");

    let mut failures = Vec::new();
    for (method, pattern, guard) in ROUTES {
        // The logout would end the shared session; its guest behaviour is
        // what the class pins.
        let path = concrete(pattern);
        let (guest, location) = send(&app, method, &path, None).await;
        let guest_ok = match guard {
            Public | Socket => {
                guest != StatusCode::UNAUTHORIZED
                    && guest != StatusCode::FORBIDDEN
                    && !(guest.is_redirection() && location == "/login")
            }
            Login | AdminPage => guest.is_redirection() && location == "/login",
            Api | Admin | Premium => guest == StatusCode::UNAUTHORIZED,
            Provider => guest.is_redirection(),
        };
        if !guest_ok {
            failures.push(format!("{method} {pattern} as guest: {guest} {location}"));
        }

        let user_check = match guard {
            Admin | Premium | AdminPage => Some(StatusCode::FORBIDDEN),
            _ => None,
        };
        if let Some(expected) = user_check {
            let (status, location) = send(&app, method, &path, Some(&user)).await;
            if status != expected {
                failures.push(format!(
                    "{method} {pattern} as a plain user: {status} {location}, expected {expected}"
                ));
            }
        }
        if matches!(guard, Login | Api) && *pattern != "/logout" {
            let (status, location) = send(&app, method, &path, Some(&user)).await;
            if status == StatusCode::UNAUTHORIZED
                || (status.is_redirection() && location == "/login")
            {
                failures.push(format!(
                    "{method} {pattern} still treats a signed-in user as a guest: {status} {location}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "guard failures:\n{}",
        failures.join("\n")
    );

    sqlx::query("delete from users where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .ok();
}
