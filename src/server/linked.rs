//! The Twitch / Discord / Patreon account-linking HTTP flows, ported from
//! the legacy `TwitchController`, `DiscordController` and
//! `PatreonController`: redirect to the provider's authorize page, and on
//! callback store the provider identity on the logged-in user's row.
//!
//! Legacy quirks ported faithfully:
//! - The routes are public, but the callbacks dereference `$request->user()`
//!   without a guard: a guest completing a link flow crashed the legacy app
//!   ("Attempt to assign property on null" - HTTP 500), so guests get a 500
//!   here too.
//! - Discord resolves the bot's private DM channel before touching the
//!   user, and a failure there was outside the controller's try/catch -
//!   also a 500.
//! - Every caught callback failure redirects to the settings page (with an
//!   error notification in legacy; flash notifications are not ported yet,
//!   like the EVE flow).

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use super::AppState;
use crate::auth::session::{
    OAUTH_STATE_COOKIE, clear_cookie, cookie_value, oauth_state_cookie, random_token,
    session_from_headers,
};

/// Where every callback ends up, success or caught failure: the legacy
/// `to_route('settings')`.
const SETTINGS_PATH: &str = "/settings";

#[derive(Deserialize, Default)]
pub struct LinkParams {
    /// The "link a different account" flag from the settings page.
    switch: Option<String>,
}

/// Laravel's `$request->boolean()` (`FILTER_VALIDATE_BOOLEAN`): "1",
/// "true", "on" and "yes" - case-insensitive, trimmed - are true;
/// anything else, including a missing parameter, is false.
fn php_request_boolean(value: Option<&str>) -> bool {
    matches!(
        value
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "on" | "yes")
    )
}

/// `GET /twitch` - redirect to Twitch. Legacy always sends `force_verify`,
/// as `'true'` only for the `?switch=` flow.
pub async fn twitch_login(
    State(state): State<AppState>,
    Query(params): Query<LinkParams>,
) -> Response {
    let force_verify = php_request_boolean(params.switch.as_deref());
    let oauth_state = random_token();
    authorize_redirect(
        &state
            .linked
            .twitch
            .authorize_url(&oauth_state, force_verify),
        &oauth_state,
    )
}

/// `GET /discord` - redirect to Discord; the `?switch=` flow asks for
/// consent again (no `prompt=none`).
pub async fn discord_login(
    State(state): State<AppState>,
    Query(params): Query<LinkParams>,
) -> Response {
    let consent = php_request_boolean(params.switch.as_deref());
    let oauth_state = random_token();
    authorize_redirect(
        &state.linked.discord.authorize_url(&oauth_state, consent),
        &oauth_state,
    )
}

/// `GET /patreon` - redirect to Patreon; no switch handling in legacy.
pub async fn patreon_login(State(state): State<AppState>) -> Response {
    let oauth_state = random_token();
    authorize_redirect(
        &state.linked.patreon.authorize_url(&oauth_state),
        &oauth_state,
    )
}

fn authorize_redirect(url: &str, oauth_state: &str) -> Response {
    let mut response = Redirect::to(url).into_response();
    append_cookie(&mut response, &oauth_state_cookie(oauth_state));
    response
}

#[derive(Deserialize, Default)]
pub struct LinkCallbackParams {
    code: Option<String>,
    state: Option<String>,
}

/// The caught-failure (and also success) redirect. The state cookie is
/// cleared on every callback outcome, like Laravel's session pull.
fn settings_redirect() -> Response {
    let mut response = Redirect::to(SETTINGS_PATH).into_response();
    append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
    response
}

/// The uncaught legacy failure paths (guest callback, Discord bot error,
/// database error): a plain 500.
fn server_error() -> Response {
    let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
    append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
    response
}

/// Socialite's `hasInvalidState`, inverted: the stored state must be
/// non-empty and equal the callback's `state` parameter.
fn state_is_valid(headers: &HeaderMap, params: &LinkCallbackParams) -> bool {
    match (cookie_value(headers, OAUTH_STATE_COOKIE), &params.state) {
        (Some(stored), Some(state)) if !stored.is_empty() => stored == *state,
        _ => false,
    }
}

/// The logged-in user for a callback, or the legacy null-dereference 500.
async fn callback_user(state: &AppState, headers: &HeaderMap) -> Result<i64, Response> {
    match session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session.user_id),
        Ok(None) => Err(server_error()),
        Err(error) => {
            eprintln!("link callback session lookup failed: {error}");
            Err(server_error())
        }
    }
}

/// `GET /twitch/callback`.
pub async fn twitch_callback(
    State(state): State<AppState>,
    Query(params): Query<LinkCallbackParams>,
    headers: HeaderMap,
) -> Response {
    if !state_is_valid(&headers, &params) {
        return settings_redirect();
    }

    // A missing code fails at the token exchange, like the legacy flow.
    let code = params.code.unwrap_or_default();
    let Ok(twitch_user) = state.linked.twitch.user(&code).await else {
        return settings_redirect();
    };

    let user_id = match callback_user(&state, &headers).await {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    // Provider ids are numeric strings stored in bigint columns; a
    // non-numeric id blew up the legacy database write the same way.
    let Ok(twitch_id) = twitch_user.id.parse::<i64>() else {
        return server_error();
    };

    let updated = sqlx::query(
        "update users
         set twitch_id = $1, twitch_name = $2, twitch_avatar = $3, twitch_email = $4,
             updated_at = now()
         where id = $5",
    )
    .bind(twitch_id)
    .bind(&twitch_user.display_name)
    .bind(&twitch_user.avatar)
    .bind(&twitch_user.email)
    .bind(user_id)
    .execute(&state.pool)
    .await;

    if let Err(error) = updated {
        eprintln!("failed to link Twitch account: {error}");
        return server_error();
    }

    settings_redirect()
}

/// `GET /discord/callback`.
pub async fn discord_callback(
    State(state): State<AppState>,
    Query(params): Query<LinkCallbackParams>,
    headers: HeaderMap,
) -> Response {
    if !state_is_valid(&headers, &params) {
        return settings_redirect();
    }

    let code = params.code.unwrap_or_default();
    let Ok(discord_user) = state.linked.discord.user(&code).await else {
        return settings_redirect();
    };

    // Legacy resolves the DM channel before reading the user, outside the
    // try/catch: failures are a 500, and guests still trigger the call.
    let Ok(channel_id) = state
        .linked
        .discord
        .private_channel_id(&discord_user.id)
        .await
    else {
        return server_error();
    };

    let user_id = match callback_user(&state, &headers).await {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    let (Ok(discord_id), Ok(channel_id)) =
        (discord_user.id.parse::<i64>(), channel_id.parse::<i64>())
    else {
        return server_error();
    };

    let updated = sqlx::query(
        "update users
         set discord_id = $1, discord_name = $2, discord_avatar = $3,
             discord_channel_id = $4, updated_at = now()
         where id = $5",
    )
    .bind(discord_id)
    .bind(&discord_user.username)
    .bind(&discord_user.avatar)
    .bind(channel_id)
    .bind(user_id)
    .execute(&state.pool)
    .await;

    if let Err(error) = updated {
        eprintln!("failed to link Discord account: {error}");
        return server_error();
    }

    settings_redirect()
}

/// `GET /patreon/callback`.
pub async fn patreon_callback(
    State(state): State<AppState>,
    Query(params): Query<LinkCallbackParams>,
    headers: HeaderMap,
) -> Response {
    if !state_is_valid(&headers, &params) {
        return settings_redirect();
    }

    let code = params.code.unwrap_or_default();
    let Ok(patreon_user) = state.linked.patreon.user(&code).await else {
        return settings_redirect();
    };

    let user_id = match callback_user(&state, &headers).await {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    let Ok(patreon_id) = patreon_user.id.parse::<i64>() else {
        return server_error();
    };

    let updated = sqlx::query(
        "update users
         set patreon_id = $1, patreon_name = $2, patreon_avatar = $3, patreon_email = $4,
             patreon_nickname = $5, updated_at = now()
         where id = $6",
    )
    .bind(patreon_id)
    .bind(&patreon_user.full_name)
    .bind(&patreon_user.avatar)
    .bind(&patreon_user.email)
    .bind(&patreon_user.nickname)
    .bind(user_id)
    .execute(&state.pool)
    .await;

    if let Err(error) = updated {
        eprintln!("failed to link Patreon account: {error}");
        return server_error();
    }

    settings_redirect()
}

fn append_cookie(response: &mut Response, cookie: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

#[cfg(test)]
mod tests {
    use super::php_request_boolean;

    #[test]
    fn request_booleans_follow_php_filter_var() {
        for truthy in ["1", "true", "TRUE", "on", "yes", " yes "] {
            assert!(php_request_boolean(Some(truthy)), "{truthy:?}");
        }
        for falsy in [
            None,
            Some(""),
            Some("0"),
            Some("false"),
            Some("2"),
            Some("switch"),
        ] {
            assert!(!php_request_boolean(falsy), "{falsy:?}");
        }
    }
}
