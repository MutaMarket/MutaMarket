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

/// `POST /public-assets` — publish an owned asset and its subtree, the
/// legacy `PublicAssetController::store`. Body: `{ "asset_id": <id> }`.
pub async fn publish_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            tracing::warn!(%error, "publish asset session lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        asset_id: Option<i64>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(asset_id) = payload.asset_id else {
        return validation_error("asset_id", "The asset id field is required.");
    };

    match crate::assets::public::publish_asset(&state.pool, session.user_id, asset_id).await {
        Ok(()) => back(&headers).into_response(),
        Err(crate::assets::public::PublishError::NotOwned) => {
            validation_error("asset_id", "The selected asset id is invalid.")
        }
        Err(error) => {
            tracing::warn!(%error, "publish asset failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// `DELETE /public-assets/{publicAsset}` — unpublish (owner only), the
/// legacy `PublicAssetController::destroy`.
pub async fn unpublish_asset(
    State(state): State<AppState>,
    axum::extract::Path(public_asset_id): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            tracing::warn!(%error, "unpublish asset session lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match crate::assets::public::unpublish_asset(&state.pool, session.user_id, public_asset_id).await
    {
        Ok(()) => back(&headers).into_response(),
        Err(crate::assets::public::PublishError::NotOwned) => StatusCode::FORBIDDEN.into_response(),
        Err(error) => {
            tracing::warn!(%error, "unpublish asset failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn back(headers: &HeaderMap) -> Redirect {
    let target = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/personal/modules");
    Redirect::to(target)
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

/// Modules per personal page, the legacy `simplePaginate(40)`.
const PERSONAL_PAGE_SIZE: i64 = 40;

/// The personal page payload shared with the Leptos server function.
pub async fn personal_page_data(
    state: &AppState,
    session: &session::Session,
) -> sqlx::Result<crate::view::personal::PersonalPageData> {
    // The active character, like the legacy getActiveCharacter(): the
    // session's choice, or the user's first character.
    let active_character: Option<i64> = match session.active_character_id {
        Some(id) => Some(id),
        None => {
            sqlx::query_scalar("select id from characters where user_id = $1 order by id limit 1")
                .bind(session.user_id)
                .fetch_optional(&state.pool)
                .await?
        }
    };

    let has_assets_scope = match active_character {
        Some(character_id) => has_assets_scope(&state.pool, character_id).await?,
        None => false,
    };

    let asset_import = crate::server::ws::latest_asset_import(
        &state.pool,
        session.user_id,
        session.active_character_id,
    )
    .await?;

    Ok(crate::view::personal::PersonalPageData {
        user_id: session.user_id,
        has_assets_scope,
        grant_scope_url: format!("/eve?scopes={}", scopes::READ_ASSETS),
        asset_import,
    })
}

/// The user's owned modules, newest first — the legacy `whereOwnedByUser`
/// scope. Legacy reads the trigger-maintained `module_ownerships` table
/// (assets plus contract items); the same union is computed directly here
/// since the trigger table is not ported.
pub async fn personal_module_entries(
    state: &AppState,
    session: &session::Session,
    query: &str,
) -> sqlx::Result<Vec<crate::view::personal::PersonalModuleEntry>> {
    // The full filter grammar applies, scoped to the account's owned
    // modules; a bad query degrades to the unfiltered set.
    let search = crate::modules::search::parse(&state.pool, &state.reference, query).await;
    let ids: Vec<i64> = match search {
        Ok(search) => {
            crate::modules::search::scoped_module_ids(
                &state.pool,
                &search,
                crate::modules::search::Scope::OwnedByUser(session.user_id),
                PERSONAL_PAGE_SIZE,
            )
            .await?
        }
        Err(crate::modules::search::SearchError::Db(error)) => return Err(error),
        Err(_) => sqlx::query_scalar(
            "select m.id from modules m
             where exists (
                       select 1 from assets a
                       join characters c on c.id = a.character_id
                       where a.item_id = m.id and a.is_abyssal and c.user_id = $1
                   )
                or exists (
                       select 1 from contract_items ci
                       join contracts ct on ct.id = ci.contract_id
                       join characters c on c.id = ct.issuer_id
                       where ci.item_id = m.id and c.user_id = $1
                   )
             order by m.id desc
             limit $2",
        )
        .bind(session.user_id)
        .bind(PERSONAL_PAGE_SIZE)
        .fetch_all(&state.pool)
        .await?,
    };

    let details =
        crate::modules::queries::details_for(&state.pool, &state.reference, ids.clone()).await?;
    let mut locations =
        crate::assets::module_locations(&state.pool, session.user_id, &ids).await?;

    Ok(details
        .into_iter()
        .map(|module| {
            let location = locations.remove(&module.id);
            crate::view::personal::PersonalModuleEntry { module, location }
        })
        .collect())
}

/// Guests get a 401 instead of the page routes' login redirect: these
/// endpoints only ever answer fetch() clients (documented divergence).
async fn require_api_session(
    pool: &sqlx::PgPool,
    headers: &HeaderMap,
) -> Result<session::Session, axum::response::Response> {
    match session::session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated.")),
        Err(error) => Err(super::api::database_error(error)),
    }
}

/// `GET /api/personal/page` — the asset import panel state.
pub async fn page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    match personal_page_data(&state, &session).await {
        Ok(data) => axum::Json(data).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `GET /api/personal/modules` — the owned module grid entries.
pub async fn modules(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<super::social::PageQueryParams>,
) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    match personal_module_entries(&state, &session, params.q.as_deref().unwrap_or("")).await {
        Ok(entries) => axum::Json(entries).into_response(),
        Err(error) => super::api::database_error(error),
    }
}
