//! The search alert routes (a rewrite addition): `GET /api/search-alerts`
//! (the account's alerts), `POST /search-alerts` (save the current
//! browser query) and `DELETE /search-alerts/{alert}`.
//!
//! The page routes follow the blocked-users pattern: guests get the
//! login redirect, failures answer JSON with Laravel-shaped statuses.
//! Saving is a premium feature: without it the save answers the API's
//! premium 403, which the bell turns into the premium pitch.
//! The query validation mirrors the module index's answers to the same
//! query (404 "Please provide a valid type." for an unknown type, 400
//! with the parser's message otherwise).

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};

use super::AppState;
use super::support::{
    db_error, error_json, require_api_session, session_or_login, validation_error,
};
use crate::modules::search::SearchError;
use crate::search_alerts::{self, CreateError, MAX_ALERTS_PER_USER};

/// `GET /api/search-alerts` — the account's alerts, newest first.
pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    match search_alerts::list(&state.pool, session.user_id).await {
        Ok(alerts) => Json(alerts).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `POST /search-alerts` — saves the `query` path as an alert: 201 with
/// the alert, or 200 with the existing one when the same normalized
/// query is already saved.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers, "search alerts").await {
        Ok(session) => session,
        Err(response) => return response,
    };

    // The premium gate, checked before validation like the legacy
    // PremiumMiddleware sits in front of a controller.
    match crate::premium::user_has_premium(&state.pool, session.user_id).await {
        Ok(true) => {}
        Ok(false) => return error_json(StatusCode::FORBIDDEN, "Premium required."),
        Err(error) => return db_error(error, "search alerts"),
    }

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        query: Option<serde_json::Value>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let query = match payload.query {
        None | Some(serde_json::Value::Null) => {
            return validation_error("query", "The query field is required.");
        }
        Some(serde_json::Value::String(query)) => query,
        Some(_) => return validation_error("query", "The query field must be a string."),
    };

    match search_alerts::create(&state.pool, &state.reference, session.user_id, &query).await {
        Ok((alert, true)) => (StatusCode::CREATED, Json(alert)).into_response(),
        Ok((alert, false)) => Json(alert).into_response(),
        Err(CreateError::MissingType) => {
            validation_error("query", "Pick a module type before saving an alert.")
        }
        Err(CreateError::TooMany) => validation_error(
            "query",
            &format!("You can save at most {MAX_ALERTS_PER_USER} search alerts."),
        ),
        Err(CreateError::Search(SearchError::TypeNotFound)) => {
            error_json(StatusCode::NOT_FOUND, "Please provide a valid type.")
        }
        Err(CreateError::Search(SearchError::Invalid(message))) => {
            error_json(StatusCode::BAD_REQUEST, &message)
        }
        Err(CreateError::Search(SearchError::Db(error))) | Err(CreateError::Db(error)) => {
            db_error(error, "search alerts")
        }
    }
}

/// `DELETE /search-alerts/{alert}` — 204 once the account's alert is
/// gone, 404 for anyone else's.
pub async fn destroy(
    State(state): State<AppState>,
    Path(alert_id): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session_or_login(&state, &headers, "search alerts").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    match search_alerts::delete(&state.pool, session.user_id, alert_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error_json(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => db_error(error, "search alerts"),
    }
}
