//! The workbench routes, the legacy `WorkbenchController`,
//! `WorkbenchModuleController` and `WorkbenchCollectionController`: a
//! per-user scratch set of modules, shareable as a `/workbench/{ids}`
//! link that any visitor can view and a signed-in visitor can import.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde_json::json;

use super::AppState;
use super::support::{back, db_error, session_or_login};
use crate::auth::session::session_from_headers;

/// Modules a shared workbench link resolves at most, the legacy
/// `workbench.max_items` config.
const WORKBENCH_MAX_ITEMS: usize = 25;

/// `GET /api/workbench` — the signed-in user's workbench with full
/// module payloads (the legacy shared Inertia `workbench` prop).
pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated."),
        Err(error) => return super::api::database_error(error),
    };

    let rows: Vec<(i64, i64)> = match sqlx::query_as(
        "select id, module_id from workbench_modules where user_id = $1 order by id",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return db_error(error, "workbench"),
    };

    let mut details = match crate::modules::queries::details_for(
        &state.pool,
        &state.reference,
        rows.iter().map(|(_, module_id)| *module_id).collect(),
    )
    .await
    {
        Ok(details) => details,
        Err(error) => return db_error(error, "workbench"),
    };
    // The legacy WorkbenchController loads withDefaultRelations, so the
    // user's notes ride along.
    if let Err(error) =
        crate::modules::queries::attach_user_notes(&state.pool, session.user_id, &mut details).await
    {
        return db_error(error, "workbench");
    }

    let entries: Vec<serde_json::Value> = details
        .into_iter()
        .map(|module| {
            let id = rows
                .iter()
                .find(|(_, module_id)| *module_id == module.id)
                .map(|(id, _)| *id);
            json!({ "id": id, "module": module })
        })
        .collect();
    axum::Json(entries).into_response()
}

/// `POST /workbench-modules` — the legacy `createOrFirst`: adding an
/// already-present module is a silent no-op.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers, "workbench").await {
        Ok(session) => session,
        Err(response) => return response,
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        module_id: Option<i64>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(module_id) = payload.module_id else {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The module id field is required.",
        );
    };

    let result = sqlx::query(
        "insert into workbench_modules (user_id, module_id)
         select $1, $2 where exists(select 1 from modules where id = $2)
         on conflict (user_id, module_id) do nothing",
    )
    .bind(session.user_id)
    .bind(module_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => back(&headers).into_response(),
        Err(error) => db_error(error, "workbench"),
    }
}

/// `PUT /workbench-modules/{workbenchModule}` — a faithful legacy
/// quirk: the update request validates nothing and updates nothing;
/// the route just redirects back.
pub async fn update(
    State(state): State<AppState>,
    axum::extract::Path(_workbench_module): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    match session_or_login(&state, &headers, "workbench").await {
        Ok(_) => back(&headers).into_response(),
        Err(response) => response,
    }
}

/// `DELETE /workbench-modules/{workbenchModule}` — owner only; the
/// legacy flashes "Unauthorized!" over a redirect, the JSON API answers
/// 403 with the same word.
pub async fn destroy(
    State(state): State<AppState>,
    axum::extract::Path(workbench_module): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session_or_login(&state, &headers, "workbench").await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let deleted = sqlx::query("delete from workbench_modules where id = $1 and user_id = $2")
        .bind(workbench_module)
        .bind(session.user_id)
        .execute(&state.pool)
        .await;
    match deleted {
        Ok(result) if result.rows_affected() > 0 => back(&headers).into_response(),
        Ok(_) => super::api::error(StatusCode::FORBIDDEN, "Unauthorized!"),
        Err(error) => db_error(error, "workbench"),
    }
}

/// `DELETE /workbench-modules/all` — clears the user's workbench.
pub async fn destroy_all(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session_or_login(&state, &headers, "workbench").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    match sqlx::query("delete from workbench_modules where user_id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await
    {
        Ok(_) => back(&headers).into_response(),
        Err(error) => db_error(error, "workbench"),
    }
}

/// Parses the shared link's `/`-separated module ids, capped like the
/// legacy `limit(max_items)`.
fn shared_ids(modules: &str) -> Vec<i64> {
    modules
        .split('/')
        .filter_map(|segment| segment.parse().ok())
        .take(WORKBENCH_MAX_ITEMS)
        .collect()
}

/// `GET /api/workbench-page/{modules}` — the shared-link view, public
/// like the legacy invitation page.
pub async fn shared(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(modules): axum::extract::Path<String>,
) -> Response {
    let mut details = match crate::modules::queries::details_for(
        &state.pool,
        &state.reference,
        shared_ids(&modules),
    )
    .await
    {
        Ok(details) => details,
        Err(error) => return db_error(error, "workbench"),
    };
    // withDefaultRelations again: signed-in visitors of a share link see
    // their own notes on the shared modules.
    if let Err(error) = super::notes::attach_notes_if_authed(&state, &headers, &mut details).await {
        return db_error(error, "workbench");
    }
    axum::Json(details).into_response()
}

/// `POST /workbench/{modules}` — the legacy invitation accept: adds the
/// shared modules the user does not already have, and reports how many
/// like the legacy flash.
pub async fn accept(
    State(state): State<AppState>,
    axum::extract::Path(modules): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    let session = match session_or_login(&state, &headers, "workbench").await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let ids = shared_ids(&modules);
    let added = sqlx::query(
        "insert into workbench_modules (user_id, module_id)
         select $1, m.id from modules m where m.id = any($2)
         on conflict (user_id, module_id) do nothing",
    )
    .bind(session.user_id)
    .bind(&ids)
    .execute(&state.pool)
    .await;

    match added {
        Ok(_) => back(&headers).into_response(),
        Err(error) => db_error(error, "workbench"),
    }
}

/// `POST /workbench-collections` — the legacy conversion: a private
/// "Workbench Collection" from the current workbench, landing on it.
pub async fn to_collection(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session_or_login(&state, &headers, "workbench").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let character_id: Option<i64> = match session.active_character_id {
        Some(id) => Some(id),
        None => {
            match sqlx::query_scalar(
                "select id from characters where user_id = $1 order by id limit 1",
            )
            .bind(session.user_id)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(id) => id,
                Err(error) => return db_error(error, "workbench"),
            }
        }
    };
    let Some(character_id) = character_id else {
        return Redirect::to("/login").into_response();
    };

    let collection = match crate::collections::create_collection(
        &state.pool,
        character_id,
        "Workbench Collection",
        None,
        "private",
    )
    .await
    {
        Ok(collection) => collection,
        Err(error) => {
            tracing::warn!(%error, "workbench collection failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let module_ids: Vec<i64> = match sqlx::query_scalar(
        "select module_id from workbench_modules where user_id = $1 order by id",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await
    {
        Ok(ids) => ids,
        Err(error) => return db_error(error, "workbench"),
    };
    for module_id in module_ids {
        if let Err(error) =
            crate::collections::add_collection_module(&state.pool, collection.id, module_id, None)
                .await
        {
            return db_error(error, "workbench");
        }
    }

    Redirect::to(&format!("/collections/{}", collection.slug())).into_response()
}
