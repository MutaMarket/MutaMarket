//! The layer that counts every request.
//!
//! It sits inside `router()` rather than around the whole `Router`,
//! because axum inserts [`MatchedPath`] before the route service runs;
//! a layer wrapped around the outside would see concrete URLs instead of
//! route patterns, which is exactly what must not be stored.

use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;

use super::{NOT_FOUND_ROUTE, ignored};
use crate::auth::session::{SESSION_COOKIE, cookie_value};
use crate::server::AppState;

pub async fn record(State(state): State<AppState>, request: Request, next: Next) -> Response {
    // Checked first, before anything is read or allocated.
    if ignored(request.uri().path()) {
        return next.run(request).await;
    }

    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| NOT_FOUND_ROUTE.to_owned());
    let label = format!("{} {route}", request.method());

    let user_id = match cookie_value(request.headers(), SESSION_COOKIE) {
        Some(token) => state.activity.resolve_user(&state.pool, &token).await,
        None => None,
    };

    let started = Instant::now();
    let response = next.run(request).await;
    state.activity.record(
        &label,
        user_id,
        response.status().as_u16(),
        started.elapsed(),
    );
    response
}
