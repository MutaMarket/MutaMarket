//! The premium endpoints (rewrite additions): gifting, where an account
//! moves whole days of one character's premium to any known character,
//! and the plan switch between plain and ad-free premium.

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use super::AppState;
use crate::auth::session;
use crate::premium::{GiftError, gift_premium, switch_plan};

#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub ad_free: bool,
}

/// `POST /premium/plan` — puts the account on the ad-free or the plain
/// plan, converting every character's remaining time, and answers with
/// the account's plan after the switch. Like the gift, the body
/// is parsed after the session check.
pub async fn plan(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };
    let body: PlanRequest = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(_) => {
            return super::api::error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Send whether the account should be ad-free.",
            );
        }
    };

    match switch_plan(&state.pool, session.user_id, body.ad_free).await {
        Ok(plan) => Json(plan).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

#[derive(Debug, Deserialize)]
pub struct GiftRequest {
    pub from_character_id: i64,
    pub to_character_name: String,
    pub days: i32,
}

/// `POST /premium/gift` — moves premium days between characters and
/// answers with both balances. The body is parsed after the session
/// check so a guest always meets the login redirect, never a 422.
pub async fn gift(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };
    let body: GiftRequest = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(_) => {
            return super::api::error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Send a donor character, a recipient name and a day count.",
            );
        }
    };

    match gift_premium(
        &state.pool,
        session.user_id,
        body.from_character_id,
        &body.to_character_name,
        body.days,
    )
    .await
    {
        Ok(outcome) => Json(outcome).into_response(),
        Err(GiftError::Db(error)) => super::api::database_error(error),
        Err(error @ GiftError::NotYours) => {
            super::api::error(StatusCode::FORBIDDEN, error.message())
        }
        Err(error) => super::api::error(StatusCode::UNPROCESSABLE_ENTITY, error.message()),
    }
}
