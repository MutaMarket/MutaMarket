//! The layer that counts every request, and captures the ones that
//! failed.
//!
//! It sits inside `router()` rather than around the whole `Router`,
//! because axum inserts [`MatchedPath`] before the route service runs;
//! a layer wrapped around the outside would see concrete URLs instead of
//! route patterns, which is exactly what must not be stored.

use std::time::Instant;

use axum::body::{Body, HttpBody};
use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;

use super::failures::{self, RequestFailure};
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
    let method = request.method().as_str().to_owned();
    let path = failures::redact_path(
        request
            .uri()
            .path_and_query()
            .map(|path| path.as_str())
            .unwrap_or(request.uri().path()),
    );

    let user_id = match cookie_value(request.headers(), SESSION_COOKIE) {
        Some(token) => state.activity.resolve_user(&state.pool, &token).await,
        None => None,
    };

    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let duration = started.elapsed();
    state.activity.record(&label, user_id, status, duration);

    if status < 400 || !state.activity.capture_allowed(&label, status) {
        return response;
    }

    let (response, body) = error_body(response).await;
    failures::record(
        &state.pool,
        &RequestFailure {
            route: label,
            method,
            path,
            status,
            user_id,
            duration,
            body,
        },
    )
    .await;
    response
}

/// Buffers a failed response's body so the capture can keep it, and
/// rebuilds the response around it.
///
/// A body whose length is not known up front, or one past the capture
/// cap, is handed back untouched and nothing is captured: the log must
/// never damage the response it describes.
async fn error_body(response: Response) -> (Response, Option<(String, i64)>) {
    let capturable = response
        .body()
        .size_hint()
        .exact()
        .is_some_and(|size| size as usize <= failures::BODY_CAPTURE_BYTES);
    if !capturable {
        return (response, None);
    }

    let (parts, body) = response.into_parts();
    match axum::body::to_bytes(body, failures::BODY_CAPTURE_BYTES).await {
        Ok(bytes) => {
            let captured = failures::truncate_body(&bytes);
            (
                Response::from_parts(parts, Body::from(bytes)),
                Some(captured),
            )
        }
        // Unreachable for the buffered bodies the size hint allows
        // through, and not worth failing the response over.
        Err(error) => {
            tracing::warn!("reading a failed response's body failed: {error}");
            (Response::from_parts(parts, Body::empty()), None)
        }
    }
}
