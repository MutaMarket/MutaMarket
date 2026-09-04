//! The premium gifting endpoint (a rewrite addition): an account moves
//! whole days of one character's premium to any known character.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use super::AppState;
use crate::auth::session;
use crate::premium::{GiftError, gift_premium};

#[derive(Debug, Deserialize)]
pub struct GiftRequest {
    pub from_character_id: i64,
    pub to_character_name: String,
    pub days: i32,
}

/// `POST /premium/gift` — moves premium days between characters and
/// answers with both balances.
pub async fn gift(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GiftRequest>,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
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
