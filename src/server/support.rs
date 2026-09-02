//! Shared handler helpers, extracted from per-module private copies:
//! the two session guards (login redirect for page-backed routes, JSON
//! 401 for fetch-only routes), the Laravel 422 validation payload, the
//! referer `back()` redirect and the generic database-error answer.
//! Behavior is byte-identical to the former copies; the per-module log
//! wording rides along as the `context` arguments.

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use sqlx::PgPool;

use super::AppState;
use crate::auth::session::{Session, session_from_headers};

/// The page-route session guard: guests are redirected to `/login`,
/// lookup failures answer a bare 500. `context` prefixes the warn line
/// ("{context} session lookup failed").
pub(super) async fn session_or_login(
    state: &AppState,
    headers: &HeaderMap,
    context: &str,
) -> Result<Session, Response> {
    match session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "{context} session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// The social/collection variant of the guard: same login redirect, but
/// a lookup failure answers 500 with the error text as the body.
pub(super) async fn require_session(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<Session, Response> {
    match session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// The optional viewer of a `withDefaultRelations` loadout: the
/// session's user when the request carries one, `None` for guests (the
/// legacy `auth()->check()` gate of `withUserNote`/`withUserAsset`).
pub(super) async fn viewer(pool: &PgPool, headers: &HeaderMap) -> sqlx::Result<Option<i64>> {
    Ok(session_from_headers(pool, headers)
        .await?
        .map(|session| session.user_id))
}

/// Guests get a 401 instead of the page routes' login redirect: these
/// endpoints only ever answer fetch() clients (documented divergence).
pub(super) async fn require_api_session(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<Session, Response> {
    match session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(super::api::error(
            StatusCode::UNAUTHORIZED,
            "Unauthenticated.",
        )),
        Err(error) => Err(super::api::database_error(error)),
    }
}

/// The session's active character, or the user's first — the legacy
/// `User::getActiveCharacter()`, which resolved the pick through the
/// account's characters on every call. The session's pick only counts
/// while the account still owns the character: a character transferred
/// or unlinked since the pick was made falls back like an empty pick.
pub(super) async fn active_character(
    pool: &PgPool,
    session: &Session,
) -> sqlx::Result<Option<i64>> {
    if let Some(id) = session.active_character_id {
        let owned: Option<i64> =
            sqlx::query_scalar("select id from characters where id = $1 and user_id = $2")
                .bind(id)
                .bind(session.user_id)
                .fetch_optional(pool)
                .await?;
        if owned.is_some() {
            return Ok(owned);
        }
    }
    sqlx::query_scalar("select id from characters where user_id = $1 order by id limit 1")
        .bind(session.user_id)
        .fetch_optional(pool)
        .await
}

/// The Laravel single-field 422 payload.
pub(super) fn validation_error(field: &str, message: &str) -> Response {
    validation_errors(serde_json::json!({ field: [message] }))
}

/// The Laravel 422 payload with a caller-built `errors` object.
pub(super) fn validation_errors(errors: serde_json::Value) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(serde_json::json!({
            "message": "The given data was invalid.",
            "errors": errors,
        })),
    )
        .into_response()
}

/// A bare `{"message": ...}` JSON error body.
pub(super) fn error_json(status: StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({ "message": crate::i18n::tr(message) })),
    )
        .into_response()
}

/// The legacy `back()`: redirect to the referer, or `/` without one.
pub(super) fn back(headers: &HeaderMap) -> Redirect {
    back_or(headers, "/")
}

/// `back()` with a route-specific fallback for referer-less requests.
pub(super) fn back_or(headers: &HeaderMap, fallback: &'static str) -> Redirect {
    Redirect::to(&back_target(headers, fallback))
}

/// The `back()` target: the Referer when it names a path on this origin
/// (a relative path, or an absolute URL on the request's own host), else
/// the fallback. A crafted header never bounces the user off-site.
pub(super) fn back_target(headers: &HeaderMap, fallback: &str) -> String {
    let referer = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if referer.starts_with('/') && !referer.starts_with("//") {
        return referer.to_owned();
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let path = referer
        .strip_prefix("https://")
        .or_else(|| referer.strip_prefix("http://"))
        .and_then(|rest| {
            let (referer_host, path) = rest.split_once('/').unwrap_or((rest, ""));
            (!host.is_empty() && referer_host.eq_ignore_ascii_case(host))
                .then(|| format!("/{path}"))
        });
    path.unwrap_or_else(|| fallback.to_owned())
}

/// Whether a request arrived from another site: the browser's
/// `Sec-Fetch-Site` says so, or its `Origin` names a host other than
/// the one serving the request (an opaque `null` origin counts as
/// foreign). Requests without either header (same-origin fetches on
/// older browsers, non-browser clients) are not cross-site.
pub(super) fn is_cross_site(headers: &HeaderMap) -> bool {
    if headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|site| site.eq_ignore_ascii_case("cross-site"))
    {
        return true;
    }
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let origin_host = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .unwrap_or_default();
    origin_host.is_empty() || !origin_host.eq_ignore_ascii_case(host)
}

/// The bare-500 database failure of the page-backed routes; `context`
/// prefixes the warn line ("{context} database error").
pub(super) fn db_error(error: sqlx::Error, context: &str) -> Response {
    tracing::warn!(%error, "{context} database error");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
