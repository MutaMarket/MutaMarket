//! The account settings page: `GET /api/settings` (page data),
//! `PUT /settings` (the notify-character pick, legacy
//! `SettingController::update`) and the linked-account visibility
//! toggles (`PUT /discord|/twitch|/patreon`, the legacy
//! Discord/Twitch/PatreonController::update).
//!
//! Deliberate divergence: the raffle system is not ported, so the
//! response carries an always-empty `raffle_wins` list and the page
//! shows the legacy empty state.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use super::AppState;
use crate::auth::session;

async fn require_api_session(
    pool: &sqlx::PgPool,
    headers: &HeaderMap,
) -> Result<session::Session, Response> {
    match session::session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated.")),
        Err(error) => Err(super::api::database_error(error)),
    }
}

/// One linked account's card data (the legacy DetailsResource shape,
/// minus the numeric id the page never used).
fn linked(name: Option<String>, avatar: Option<String>, is_public: bool) -> serde_json::Value {
    match name {
        Some(name) => json!({ "name": name, "avatar": avatar, "is_public": is_public }),
        None => serde_json::Value::Null,
    }
}

/// `GET /api/settings` — everything the settings page shows: the
/// account's characters, the explicit notify pick (null means the
/// notification sender falls back to the first character), and the
/// three linked-account cards.
pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let characters: Result<Vec<(i64, String)>, _> =
        sqlx::query_as("select id, name from characters where user_id = $1 order by id")
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await;
    let characters = match characters {
        Ok(characters) => characters,
        Err(error) => return super::api::database_error(error),
    };

    let notify: Result<Option<(i64, String)>, _> = sqlx::query_as(
        "select c.id, c.name from notify_characters nc
         join characters c on c.id = nc.character_id
         where nc.user_id = $1",
    )
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await;
    let notify = match notify {
        Ok(notify) => notify,
        Err(error) => return super::api::database_error(error),
    };

    type UserRow = (
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        Option<String>,
        bool,
    );
    let user: Result<UserRow, _> = sqlx::query_as(
        "select discord_name, discord_avatar, discord_is_public,
                twitch_name, twitch_avatar, twitch_is_public,
                patreon_name, patreon_avatar, patreon_is_public
         from users where id = $1",
    )
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await;
    let (
        discord_name,
        discord_avatar,
        discord_public,
        twitch_name,
        twitch_avatar,
        twitch_public,
        patreon_name,
        patreon_avatar,
        patreon_public,
    ) = match user {
        Ok(user) => user,
        Err(error) => return super::api::database_error(error),
    };

    axum::Json(json!({
        "characters": characters
            .iter()
            .map(|(id, name)| json!({ "id": id, "name": name }))
            .collect::<Vec<_>>(),
        "character_to_notify": notify.map(|(id, name)| json!({ "id": id, "name": name })),
        "discord": linked(discord_name, discord_avatar, discord_public),
        "twitch": linked(twitch_name, twitch_avatar, twitch_public),
        "patreon": linked(patreon_name, patreon_avatar, patreon_public),
        // The raffle system is not ported; the card shows its empty state.
        "raffle_wins": [],
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct UpdateParams {
    pub character_to_notify: Option<i64>,
}

/// `PUT /settings` — the legacy `SettingController::update`: validate
/// the pick against the user's own characters, steal the character
/// from any other account's pick (legacy delete), and upsert the row.
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<UpdateParams>,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };

    let owned = match params.character_to_notify {
        Some(character_id) => {
            match sqlx::query_scalar::<_, bool>(
                "select exists (select 1 from characters where id = $1 and user_id = $2)",
            )
            .bind(character_id)
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await
            {
                Ok(owned) => owned,
                Err(error) => return super::api::database_error(error),
            }
        }
        None => false,
    };
    // The legacy exists/in validation message.
    if !owned {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The selected character to notify is invalid.",
        );
    }
    let character_id = params.character_to_notify.expect("validated above");

    let result: sqlx::Result<()> = async {
        let mut tx = state.pool.begin().await?;
        sqlx::query("delete from notify_characters where character_id = $1 and user_id != $2")
            .bind(character_id)
            .bind(session.user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "insert into notify_characters (user_id, character_id)
             values ($1, $2)
             on conflict (user_id) do update
             set character_id = excluded.character_id, updated_at = now()",
        )
        .bind(session.user_id)
        .bind(character_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }
    .await;

    match result {
        Ok(()) => axum::Json(json!({ "message": "Your settings have been updated." }))
            .into_response(),
        Err(error) => super::api::database_error(error),
    }
}

#[derive(Debug, Deserialize)]
pub struct VisibilityParams {
    pub is_public: Option<String>,
}

/// The `required|boolean` Laravel rule: true/false/1/0 in any casing
/// the frontend sends; anything else is invalid.
fn parse_boolean(value: Option<&str>) -> Option<bool> {
    match value {
        Some("1") | Some("true") => Some(true),
        Some("0") | Some("false") => Some(false),
        _ => None,
    }
}

async fn set_visibility(
    state: &AppState,
    headers: &HeaderMap,
    params: &VisibilityParams,
    column: &str,
) -> Response {
    let session = match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };
    let Some(is_public) = parse_boolean(params.is_public.as_deref()) else {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The is public field is required.",
        );
    };
    // The column is one of our three literals, never user input.
    let result = sqlx::query(&format!("update users set {column} = $1 where id = $2"))
        .bind(is_public)
        .bind(session.user_id)
        .execute(&state.pool)
        .await;
    match result {
        // The legacy controllers redirect to the settings page.
        Ok(_) => Redirect::to("/settings").into_response(),
        Err(error) => super::api::database_error(error),
    }
}

pub async fn update_discord(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<VisibilityParams>,
) -> Response {
    set_visibility(&state, &headers, &params, "discord_is_public").await
}

pub async fn update_twitch(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<VisibilityParams>,
) -> Response {
    set_visibility(&state, &headers, &params, "twitch_is_public").await
}

pub async fn update_patreon(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<VisibilityParams>,
) -> Response {
    set_visibility(&state, &headers, &params, "patreon_is_public").await
}
