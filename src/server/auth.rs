//! The EVE SSO HTTP flow, ported from the legacy `EveController` +
//! `EsiAuthService`: redirect to EVE with the requested scopes, and on
//! callback resolve the character to an account via the owner hash, store
//! the token, and open a session.

use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use super::AppState;
use crate::auth::session::{
    self, OAUTH_STATE_COOKIE, SESSION_COOKIE, clear_cookie, cookie_value, oauth_state_cookie,
    random_token, session_cookie,
};
use crate::auth::{scopes, sso::VerifiedCharacter};

#[derive(Deserialize, Default)]
pub struct LoginParams {
    /// Space-separated scope override, like the legacy `scopes` query param.
    scopes: Option<String>,
    without_scopes: Option<bool>,
    /// Marks the flow as adding a character to the logged-in account, like
    /// the legacy `?add_to_account` session flag.
    add_to_account: Option<bool>,
}

/// Marker cookie for the add-character flow; the callback attaches the new
/// character to the session's account instead of resolving by owner hash.
const ADD_TO_ACCOUNT_COOKIE: &str = "mm_add_account";
/// Marks the /eve/admin authorize flow: the callback registers the
/// character as the service character instead of logging it in.
const SERVICE_AUTHORIZE_COOKIE: &str = "mm_service_auth";

/// `GET /eve` — redirect to EVE's SSO authorize page.
pub async fn eve_login(State(state): State<AppState>, Query(params): Query<LoginParams>) -> Response {
    let requested = if params.without_scopes.unwrap_or(false) {
        Vec::new()
    } else if let Some(custom) = &params.scopes {
        custom.split_whitespace().collect()
    } else {
        scopes::DEFAULT_LOGIN.to_vec()
    };

    let mut response = authorize_redirect(&state, &requested);
    if params.add_to_account.unwrap_or(false) {
        // Like the legacy session flag: the actual account comes from the
        // session at callback time, so the cookie is only a marker.
        append_cookie(
            &mut response,
            &format!("{ADD_TO_ACCOUNT_COOKIE}=1; Path=/; HttpOnly; SameSite=Lax; Max-Age=600"),
        );
    }
    response
}

/// `GET /eve/corporation` — the normal login with the corporation assets
/// scope, like the legacy `CorporationScopeController`.
pub async fn eve_login_corporation() -> Redirect {
    Redirect::to(&format!("/eve?scopes={}", scopes::READ_CORPORATION_ASSETS))
}

/// `GET /eve/admin` — the service-character authorize flow: every scope
/// the background features need. The callback stores the character and
/// its tokens and registers it as the service character (structure
/// resolution, and donation processing when it lands) — the admin's own
/// session stays untouched.
pub async fn eve_login_admin(State(state): State<AppState>) -> Response {
    let mut response = authorize_redirect(&state, &scopes::ADMIN_LOGIN);
    append_cookie(
        &mut response,
        &format!("{SERVICE_AUTHORIZE_COOKIE}=1; Path=/; HttpOnly; SameSite=Lax; Max-Age=600"),
    );
    response
}

fn authorize_redirect(state: &AppState, requested_scopes: &[&str]) -> Response {
    let oauth_state = random_token();
    let url = state.sso.authorize_url(&oauth_state, requested_scopes);

    let mut response = Redirect::to(&url).into_response();
    append_cookie(&mut response, &oauth_state_cookie(&oauth_state));
    response
}

#[derive(Deserialize, Default)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
}

/// `GET /eve/callback` — the SSO return leg. Any failure falls back to the
/// home page, like the legacy controller's error notification path.
pub async fn eve_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Response {
    let failure = || {
        let mut response = Redirect::to("/").into_response();
        append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
        response
    };

    let (Some(code), Some(callback_state)) = (&params.code, &params.state) else {
        return failure();
    };

    // The state must match the one issued on the way out.
    if cookie_value(&headers, OAUTH_STATE_COOKIE).as_deref() != Some(callback_state) {
        return failure();
    }

    let Ok(tokens) = state.sso.exchange_code(code).await else {
        return failure();
    };
    let Ok(character) = state.sso.verify(&tokens.access_token).await else {
        return failure();
    };

    let Ok(affiliations) = state.esi.affiliations(&[character.character_id]).await else {
        return failure();
    };
    let Some(affiliation) = affiliations.first() else {
        return failure();
    };

    // The service-authorize flow: an admin's session registers the
    // authenticated character as the service character; tokens are
    // stored, no login happens, and the admin returns to the dashboard.
    // Without an admin session the marker is ignored and the flow falls
    // through to a plain login (the legacy /eve/admin behavior).
    if cookie_value(&headers, SERVICE_AUTHORIZE_COOKIE).is_some()
        && let Ok(Some(session)) =
            crate::auth::session::session_from_headers(&state.pool, &headers).await
        && let Ok(true) = sqlx::query_scalar::<_, bool>("select is_admin from users where id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await
    {
        if store_service_character(&state, &character, &tokens, affiliation).await.is_err() {
            return failure();
        }
        let mut response = Redirect::to("/admin").into_response();
        append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
        append_cookie(&mut response, &clear_cookie(SERVICE_AUTHORIZE_COOKIE));
        return response;
    }

    // The add-character flow attaches to the logged-in account instead of
    // resolving by owner hash (legacy `EsiAuthService::addToAccount`).
    if cookie_value(&headers, ADD_TO_ACCOUNT_COOKIE).is_some()
        && let Ok(Some(session)) =
            crate::auth::session::session_from_headers(&state.pool, &headers).await
    {
        let added =
            add_character_to_account(&state, session.user_id, &character, &tokens, affiliation)
                .await;
        if added.is_err() {
            return failure();
        }
        // Acting as the new character, like the legacy setActiveCharacter.
        let _ = sqlx::query("update sessions set active_character_id = $1 where token = $2")
            .bind(character.character_id)
            .bind(&session.token)
            .execute(&state.pool)
            .await;

        let mut response = Redirect::to("/").into_response();
        append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
        append_cookie(&mut response, &clear_cookie(ADD_TO_ACCOUNT_COOKIE));
        return response;
    }

    let login = log_in_character(&state, &character, &tokens, affiliation).await;

    let Ok(session_token) = login else {
        return failure();
    };

    let mut response = Redirect::to("/").into_response();
    append_cookie(&mut response, &session_cookie(&session_token));
    append_cookie(&mut response, &clear_cookie(OAUTH_STATE_COOKIE));
    append_cookie(&mut response, &clear_cookie(ADD_TO_ACCOUNT_COOKIE));
    append_cookie(&mut response, &clear_cookie(SERVICE_AUTHORIZE_COOKIE));
    response
}

/// Upserts the character and its token, then attaches it to the given
/// account regardless of owner hash, like the legacy `addToAccount`.
/// Persists the service character and its tokens without touching any
/// account, then records it as the app's service character.
async fn store_service_character(
    state: &AppState,
    character: &VerifiedCharacter,
    tokens: &crate::auth::sso::SsoTokens,
    affiliation: &crate::esi::EsiAffiliation,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into characters (id, name, corporation_id, alliance_id)
         values ($1, $2, $3, $4)
         on conflict (id) do update set
             name = excluded.name,
             corporation_id = excluded.corporation_id,
             alliance_id = excluded.alliance_id,
             updated_at = now()",
    )
    .bind(character.character_id)
    .bind(&character.character_name)
    .bind(affiliation.corporation_id)
    .bind(affiliation.alliance_id)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7))",
    )
    .bind(character.character_id)
    .bind(&tokens.access_token)
    .bind(&tokens.refresh_token)
    .bind(&tokens.token_type)
    .bind(&character.character_owner_hash)
    .bind(&character.scopes)
    .bind(tokens.expires_in as f64)
    .execute(&state.pool)
    .await?;

    crate::app_settings::set(
        &state.pool,
        crate::app_settings::SERVICE_CHARACTER_KEY,
        &character.character_id.to_string(),
    )
    .await
}

async fn add_character_to_account(
    state: &AppState,
    user_id: i64,
    character: &VerifiedCharacter,
    tokens: &crate::auth::sso::SsoTokens,
    affiliation: &crate::esi::EsiAffiliation,
) -> Result<(), sqlx::Error> {
    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "insert into characters (id, name, corporation_id, alliance_id)
         values ($1, $2, $3, $4)
         on conflict (id) do update set
             name = excluded.name,
             corporation_id = excluded.corporation_id,
             alliance_id = excluded.alliance_id,
             updated_at = now()",
    )
    .bind(character.character_id)
    .bind(&character.character_name)
    .bind(affiliation.corporation_id)
    .bind(affiliation.alliance_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7))",
    )
    .bind(character.character_id)
    .bind(&tokens.access_token)
    .bind(&tokens.refresh_token)
    .bind(&tokens.token_type)
    .bind(&character.character_owner_hash)
    .bind(&character.scopes)
    .bind(tokens.expires_in as f64)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "update characters set user_id = $1, character_owner_hash = $2 where id = $3",
    )
    .bind(user_id)
    .bind(&character.character_owner_hash)
    .bind(character.character_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

/// `PUT /auth/character/{character}` — act as another owned character, the
/// legacy `UserCharacterController::update` (403 on foreign characters).
pub async fn switch_character(
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(Some(session)) =
        crate::auth::session::session_from_headers(&state.pool, &headers).await
    else {
        return Redirect::to("/login").into_response();
    };
    let Some(character_id) = crate::characters::character_id_from_slug(&slug) else {
        return axum::http::StatusCode::NOT_FOUND.into_response();
    };

    let owned: bool = sqlx::query_scalar(
        "select exists (select 1 from characters where id = $1 and user_id = $2)",
    )
    .bind(character_id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);
    if !owned {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }

    if let Err(error) = sqlx::query("update sessions set active_character_id = $1 where token = $2")
        .bind(character_id)
        .bind(&session.token)
        .execute(&state.pool)
        .await
    {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
    }

    let back = headers
        .get(axum::http::header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");
    Redirect::to(back).into_response()
}

/// `DELETE /auth/character/{character}` — unlink a character, the legacy
/// `UserCharacterController::destroy` guards: never someone else's, never
/// the last one; removing the active character switches to the first
/// remaining.
pub async fn remove_character(
    State(state): State<AppState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(Some(session)) =
        crate::auth::session::session_from_headers(&state.pool, &headers).await
    else {
        return Redirect::to("/login").into_response();
    };
    let Some(character_id) = crate::characters::character_id_from_slug(&slug) else {
        return Redirect::to("/").into_response();
    };

    let owned: bool = sqlx::query_scalar(
        "select exists (select 1 from characters where id = $1 and user_id = $2)",
    )
    .bind(character_id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(false);
    let count: i64 = sqlx::query_scalar("select count(*) from characters where user_id = $1")
        .bind(session.user_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    // Legacy answers both refusals with a redirect home (error toast only).
    if !owned || count <= 1 {
        return Redirect::to("/").into_response();
    }

    if let Err(error) = sqlx::query("update characters set user_id = null where id = $1")
        .bind(character_id)
        .execute(&state.pool)
        .await
    {
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
    }

    if session.active_character_id == Some(character_id) {
        let _ = sqlx::query(
            "update sessions set active_character_id =
                 (select id from characters where user_id = $1 order by id limit 1)
             where token = $2",
        )
        .bind(session.user_id)
        .bind(&session.token)
        .execute(&state.pool)
        .await;
    }

    Redirect::to("/").into_response()
}

/// Upserts the character, stores the token, resolves the account via the
/// owner hash (legacy `EsiAuthService::getUser`), and opens a session.
async fn log_in_character(
    state: &AppState,
    character: &VerifiedCharacter,
    tokens: &crate::auth::sso::SsoTokens,
    affiliation: &crate::esi::EsiAffiliation,
) -> Result<String, sqlx::Error> {
    let mut tx = state.pool.begin().await?;

    // The owner hash is deliberately not touched here: it participates in
    // the account resolution below.
    sqlx::query(
        "insert into characters (id, name, corporation_id, alliance_id)
         values ($1, $2, $3, $4)
         on conflict (id) do update set
             name = excluded.name,
             corporation_id = excluded.corporation_id,
             alliance_id = excluded.alliance_id,
             updated_at = now()",
    )
    .bind(character.character_id)
    .bind(&character.character_name)
    .bind(affiliation.corporation_id)
    .bind(affiliation.alliance_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7))",
    )
    .bind(character.character_id)
    .bind(&tokens.access_token)
    .bind(&tokens.refresh_token)
    .bind(&tokens.token_type)
    .bind(&character.character_owner_hash)
    .bind(&character.scopes)
    .bind(tokens.expires_in as f64)
    .execute(&mut *tx)
    .await?;

    let existing: Option<(Option<i64>, Option<String>)> = sqlx::query_as(
        "select user_id, character_owner_hash from characters where id = $1",
    )
    .bind(character.character_id)
    .fetch_optional(&mut *tx)
    .await?;

    // Same user and unchanged owner hash: log into the existing account.
    // Anything else (first login, or the character was transferred) gets a
    // fresh account owning the character.
    let previous_user_id = existing.as_ref().and_then(|(user_id, _)| *user_id);
    let user_id = match existing {
        Some((Some(user_id), Some(stored_hash))) if stored_hash == character.character_owner_hash => {
            user_id
        }
        _ => {
            let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
                .bind(&character.character_name)
                .fetch_one(&mut *tx)
                .await?;

            sqlx::query(
                "update characters set user_id = $1, character_owner_hash = $2, updated_at = now()
                 where id = $3",
            )
            .bind(user_id)
            .bind(&character.character_owner_hash)
            .bind(character.character_id)
            .execute(&mut *tx)
            .await?;

            // The character moved to a new account; an old account left
            // without any characters is deleted, like the legacy cleanup.
            if let Some(previous) = previous_user_id.filter(|previous| *previous != user_id) {
                sqlx::query(
                    "delete from users
                     where id = $1
                       and not exists (select 1 from characters where user_id = $1)",
                )
                .bind(previous)
                .execute(&mut *tx)
                .await?;
            }

            user_id
        }
    };

    tx.commit().await?;

    session::create_session(&state.pool, user_id, Some(character.character_id)).await
}

/// `POST /logout` — destroy the session; guests get the login redirect the
/// auth middleware would give them.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(token) = cookie_value(&headers, SESSION_COOKIE) else {
        return Redirect::to("/login").into_response();
    };

    if let Err(error) = session::delete_session(&state.pool, &token).await {
        eprintln!("logout failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let mut response = Redirect::to("/").into_response();
    append_cookie(&mut response, &clear_cookie(SESSION_COOKIE));
    response
}

fn append_cookie(response: &mut Response, cookie: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}
