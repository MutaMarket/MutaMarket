//! `POST /estimate/{module}` — on-demand estimate refresh, the legacy
//! `EstimatorController::update`.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::auth::session;
use crate::estimator;

/// Runs the single-module estimate synchronously and redirects back.
///
/// The legacy controller calls the `app:estimate` command and redirects
/// back with an "Estimate updated" notification even when the estimation
/// itself reported failure — the command's exit code is ignored, only a
/// thrown exception switches the notification to "Estimate failed", and
/// both outcomes are the same redirect back. Flash notifications are not
/// ported yet, so both outcomes are the bare redirect here.
pub async fn update(
    State(state): State<AppState>,
    Path(module): Path<String>,
    headers: HeaderMap,
) -> Response {
    // The route sits behind the auth middleware: guests get the login
    // redirect.
    match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(_)) => {}
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("estimate session lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // Legacy implicit route binding: an unknown module id 404s before the
    // controller runs (a non-numeric id matches nothing either).
    let Ok(module_id) = module.parse::<i64>() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let known: Result<Option<i64>, sqlx::Error> =
        sqlx::query_scalar("select id from modules where id = $1")
            .bind(module_id)
            .fetch_optional(&state.pool)
            .await;

    match known {
        Ok(Some(_)) => {}
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            eprintln!("estimate module lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    if let Err(error) =
        estimator::estimate_module_value(&state.pool, &state.estimator, module_id).await
    {
        eprintln!("estimate for module {module_id} failed: {error}");
    }

    // back(): the previous page from the Referer header, falling back to
    // home like Laravel's fallback.
    let back = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");

    Redirect::to(back).into_response()
}
