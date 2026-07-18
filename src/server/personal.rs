//! `POST /personal/modules` — start the user's asset imports, the legacy
//! `PersonalModuleController::store`.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::auth::scopes;
use crate::auth::session;

/// Starts an asset import for every character of the logged-in user and
/// redirects back.
///
/// Legacy quirks ported faithfully:
/// - only the *active* character's token is checked for the Read Assets
///   scope, but imports are dispatched for **all** characters — ones
///   without the scope simply produce failed import rows;
/// - the missing-scope response is also just a redirect back (legacy adds
///   an error notification with a "Grant ESI scope" CTA; flash
///   notifications are not ported yet, so the page surfaces the grant
///   link inline instead).
pub async fn store(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("personal modules session lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // back(): the previous page from the Referer header, falling back to
    // the personal modules page (the legacy setIntendedUrl fallback).
    let back = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/personal/modules")
        .to_owned();

    let characters: Vec<i64> =
        match sqlx::query_scalar("select id from characters where user_id = $1 order by id")
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await
        {
            Ok(characters) => characters,
            Err(error) => {
                eprintln!("personal modules character lookup failed: {error}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    // The active character, like the legacy getActiveCharacter(): the
    // session's choice, or the user's first character.
    let Some(active_character) = session.active_character_id.or(characters.first().copied()) else {
        return Redirect::to(&back).into_response();
    };

    match has_assets_scope(&state.pool, active_character).await {
        Ok(true) => {}
        Ok(false) => return Redirect::to(&back).into_response(),
        Err(error) => {
            eprintln!("personal modules scope lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // The legacy dispatches one queued AssetImport job per character; the
    // equivalent here is a background task per character running the
    // ported sync (which creates and advances the import row the
    // WebSocket progress stream watches).
    for character_id in characters {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = crate::assets::sync_character_assets(
                &state.pool,
                &state.reference,
                &state.esi,
                &state.sso,
                &state.estimator,
                character_id,
            )
            .await
            {
                eprintln!("requested asset import for character {character_id} failed: {error}");
            }
        });
    }

    Redirect::to(&back).into_response()
}

/// Whether the character holds an ESI token with the Read Assets scope,
/// the legacy `hasEsiTokenWithScope(EsiScope::ReadAssets)`.
pub async fn has_assets_scope(
    pool: &sqlx::PgPool,
    character_id: i64,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "select exists(
             select 1 from esi_tokens
             where character_id = $1 and $2 = any(scopes)
         )",
    )
    .bind(character_id)
    .bind(scopes::READ_ASSETS)
    .fetch_one(pool)
    .await
}
