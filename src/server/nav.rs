//! `GET /api/nav-state` — the navigation payload: the session user plus
//! the account's characters in one round trip, like the legacy
//! `auth.user` shared Inertia prop with its `characters` relation.

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use sqlx::PgPool;

use super::AppState;
use crate::auth::scopes;
use crate::auth::session::{Session, session_from_headers};
use crate::view::nav::{AccountCharacter, CurrentUser, NavState};

/// Guests get a JSON `null`, mirroring the legacy shared prop where
/// `auth.user` is null for guests.
pub async fn show(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Json(serde_json::Value::Null).into_response(),
        Err(error) => return super::api::database_error(error),
    };

    match nav_state(&state.pool, &session).await {
        Ok(Some(nav)) => Json(nav).into_response(),
        Ok(None) => Json(serde_json::Value::Null).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

pub async fn nav_state(pool: &PgPool, session: &Session) -> sqlx::Result<Option<NavState>> {
    let Some(user) = current_user(pool, session).await? else {
        return Ok(None);
    };
    let characters = account_characters(pool, session).await?;

    Ok(Some(NavState { user, characters }))
}

/// The logged-in user of the session, if it still resolves to a user row.
pub async fn current_user(pool: &PgPool, session: &Session) -> sqlx::Result<Option<CurrentUser>> {
    let user: Option<(String, bool, bool)> = sqlx::query_as(
        "select name, is_admin,
                exists (select 1 from characters c
                        where c.user_id = users.id
                          and c.premium_paid_until > now()) as has_premium
         from users where id = $1",
    )
    .bind(session.user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user.map(|(name, is_admin, has_premium)| CurrentUser {
        name,
        active_character_id: session.active_character_id,
        is_admin,
        has_premium,
    }))
}

/// The session user's characters with the active flag and asset-scope
/// state, like the legacy `auth.user.characters` page prop.
pub async fn account_characters(
    pool: &PgPool,
    session: &Session,
) -> sqlx::Result<Vec<AccountCharacter>> {
    let rows: Vec<(i64, String, Option<i64>, bool)> = sqlx::query_as(
        "select c.id, c.name, c.corporation_id,
                exists (select 1 from esi_tokens t
                        where t.character_id = c.id and $1 = any(t.scopes)) as has_asset_token
         from characters c where c.user_id = $2 order by c.id",
    )
    .bind(scopes::READ_ASSETS)
    .bind(session.user_id)
    .fetch_all(pool)
    .await?;

    // The active character falls back to the first one, like the legacy
    // getActiveCharacter.
    let active_id = session
        .active_character_id
        .filter(|id| rows.iter().any(|(row_id, ..)| row_id == id))
        .or_else(|| rows.first().map(|(id, ..)| *id));

    Ok(rows
        .into_iter()
        .map(|(id, name, corporation_id, has_asset_token)| AccountCharacter {
            id,
            name,
            corporation_id,
            has_asset_token,
            active: Some(id) == active_id,
        })
        .collect())
}
