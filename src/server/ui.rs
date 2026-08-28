//! The in-game UI route: `POST /ui/contract` (legacy
//! `UIController::openContract`).
//!
//! Divergence, as documented in `server::offers`: the legacy answered
//! `back()->notify(...)` flash toasts; the fetch-driven frontend gets the
//! bare referer redirect on success and JSON statuses with the legacy
//! texts on failure. The missing-scope notify carried an action link to
//! the SSO grant; that URL rides in `grant_scope_url` like the personal
//! page payload.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::auth::scopes;
use crate::auth::session;

async fn session_or_login(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<session::Session, Response> {
    match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "ui session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

fn validation_error(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(serde_json::json!({
            "message": "The given data was invalid.",
            "errors": { field: [message] },
        })),
    )
        .into_response()
}

fn back(headers: &HeaderMap) -> Redirect {
    let target = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");
    Redirect::to(target)
}

fn db_error(error: sqlx::Error) -> Response {
    tracing::warn!(%error, "ui contract database error");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// The failed-to-open answer, the legacy error notify texts as a 502
/// (the failure lives upstream in ESI; documented divergence from the
/// legacy redirect-with-toast).
fn open_failed() -> Response {
    (
        StatusCode::BAD_GATEWAY,
        axum::Json(serde_json::json!({
            "message": "An error occurred while trying to open the contract in the EVE Online client.",
        })),
    )
        .into_response()
}

/// `POST /ui/contract` — opens a contract window in the EVE client via
/// ESI, guarded by the OpenWindow scope.
pub async fn open_contract(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    // contract_id: required|integer|exists:contracts,id.
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    let contract_id_value = &payload["contract_id"];
    if contract_id_value.is_null() {
        return validation_error("contract_id", "The contract id field is required.");
    }
    let contract_id = match contract_id_value {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(string) => string.parse().ok(),
        _ => None,
    };
    let Some(contract_id) = contract_id else {
        return validation_error("contract_id", "The contract id field must be an integer.");
    };
    let contract_exists: bool =
        match sqlx::query_scalar("select exists(select 1 from contracts where id = $1)")
            .bind(contract_id)
            .fetch_one(&state.pool)
            .await
        {
            Ok(exists) => exists,
            Err(error) => return db_error(error),
        };
    if !contract_exists {
        return validation_error("contract_id", "The selected contract id is invalid.");
    }

    // The active character, like the legacy getActiveCharacter(). An
    // account with no characters null-crashed the legacy controller — the
    // same 500 here.
    let character: Option<i64> = match session.active_character_id {
        Some(id) => Some(id),
        None => {
            match sqlx::query_scalar(
                "select id from characters where user_id = $1 order by id limit 1",
            )
            .bind(session.user_id)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(character) => character,
                Err(error) => return db_error(error),
            }
        }
    };
    let Some(character_id) = character else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    // hasEsiTokenWithScope(EsiScope::OpenWindow): without it, the legacy
    // notify pointed at the SSO grant URL.
    let has_scope: bool = match sqlx::query_scalar(
        "select exists(select 1 from esi_tokens where character_id = $1 and $2 = any(scopes))",
    )
    .bind(character_id)
    .bind(scopes::OPEN_WINDOW)
    .fetch_one(&state.pool)
    .await
    {
        Ok(has_scope) => has_scope,
        Err(error) => return db_error(error),
    };
    if !has_scope {
        // The legacy message, typo included ("th contract ingame").
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "message":
                    "You need to grant the \"Open Window\" ESI scope to open th contract ingame!",
                "grant_scope_url": format!("/eve?scopes={}", scopes::OPEN_WINDOW),
            })),
        )
            .into_response();
    }

    // The legacy wraps the ESI call in a catch-all: any token or
    // transport failure reports the same failed notify.
    let token = match crate::auth::tokens::valid_access_token(
        &state.pool,
        &state.sso,
        character_id,
        scopes::OPEN_WINDOW,
    )
    .await
    {
        Ok(Some(token)) => token,
        Ok(None) => return open_failed(),
        Err(error) => {
            tracing::warn!(%error, character_id, "open-contract token acquisition failed");
            return open_failed();
        }
    };

    match state.esi.open_contract_window(&token.access_token, contract_id).await {
        Ok(()) => back(&headers).into_response(),
        Err(crate::esi::EsiError::Forbidden(status)) => {
            // ESI rejected the token: drop it like the legacy connector's
            // handleFailedResponse, then report the failure.
            tracing::warn!(%status, character_id, "open-contract token rejected");
            if let Err(error) =
                crate::auth::tokens::delete_token(&state.pool, token.token_id).await
            {
                tracing::warn!(%error, "deleting the rejected token failed");
            }
            open_failed()
        }
        Err(error) => {
            tracing::warn!(%error, contract_id, "open-contract ESI call failed");
            open_failed()
        }
    }
}
