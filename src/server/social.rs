//! Collections and character mutation endpoints plus the OpenGraph image
//! routes, ported from the legacy Collection/CollectionModule/Character/
//! OpenGraph controllers. Success responses are Inertia-style redirects
//! (302 to the target or back to the referer); invalid payloads answer the
//! Laravel 422 shape.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::auth::session::{Session, session_from_headers};
use crate::collections::{self, COLLECTION_VISIBILITIES};

/// Longest collection name/description, the legacy max:255.
const COLLECTION_TEXT_MAX: usize = 255;

/// Longest character description, the legacy max:5000.
const CHARACTER_DESCRIPTION_MAX: usize = 5000;

fn back(headers: &HeaderMap) -> Redirect {
    Redirect::to(
        headers.get(header::REFERER).and_then(|value| value.to_str().ok()).unwrap_or("/"),
    )
}

fn validation_error(errors: serde_json::Value) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "message": "The given data was invalid.", "errors": errors })),
    )
        .into_response()
}

async fn require_session(pool: &PgPool, headers: &HeaderMap) -> Result<Session, Response> {
    match session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()),
    }
}

/// The session's active character, like `User::getActiveCharacter` (the
/// explicit active character or any owned one).
async fn active_character(pool: &PgPool, session: &Session) -> sqlx::Result<Option<i64>> {
    if let Some(character_id) = session.active_character_id {
        return Ok(Some(character_id));
    }

    sqlx::query_scalar("select id from characters where user_id = $1 order by id limit 1")
        .bind(session.user_id)
        .fetch_optional(pool)
        .await
}

#[derive(Deserialize, Default)]
struct CollectionPayload {
    name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    #[serde(default)]
    modules: Option<Vec<serde_json::Value>>,
}

#[allow(clippy::result_large_err)] // the error is the ready-to-send 422 response
fn validate_collection(payload: &CollectionPayload) -> Result<(), Response> {
    let mut errors = serde_json::Map::new();

    match &payload.name {
        Some(name) if !name.is_empty() && name.len() <= COLLECTION_TEXT_MAX => {}
        _ => {
            errors.insert("name".into(), json!(["The name field is required."]));
        }
    }
    if payload.description.as_ref().is_some_and(|d| d.len() > COLLECTION_TEXT_MAX) {
        errors.insert(
            "description".into(),
            json!(["The description field must not be greater than 255 characters."]),
        );
    }
    match payload.visibility.as_deref() {
        Some(visibility) if COLLECTION_VISIBILITIES.contains(&visibility) => {}
        _ => {
            errors.insert("visibility".into(), json!(["The selected visibility is invalid."]));
        }
    }

    if errors.is_empty() { Ok(()) } else { Err(validation_error(json!(errors))) }
}

/// `POST /collections`
pub async fn store_collection(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let session = match require_session(&pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let payload: CollectionPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_collection(&payload) {
        return response;
    }
    let Ok(Some(character_id)) = active_character(&pool, &session).await else {
        return back(&headers).into_response();
    };

    match collections::create_collection(
        &pool,
        character_id,
        payload.name.as_deref().unwrap_or_default(),
        payload.description.as_deref(),
        payload.visibility.as_deref().unwrap_or_default(),
    )
    .await
    {
        Ok(collection) => Redirect::to(&format!("/collections/{}", collection.slug())).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// `POST /collections/modules` — create a collection and fill it, the
/// legacy storeAndAddModules (description required here, and every module
/// id must exist).
pub async fn store_collection_with_modules(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let session = match require_session(&pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let payload: CollectionPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_collection(&payload) {
        return response;
    }
    if payload.description.is_none() {
        return validation_error(json!({"description": ["The description field is required."]}));
    }
    let Some(module_values) = payload.modules.as_ref().filter(|modules| !modules.is_empty()) else {
        return validation_error(json!({"modules": ["The modules field is required."]}));
    };
    let module_ids: Vec<i64> =
        module_values.iter().filter_map(serde_json::Value::as_i64).collect();
    let known: i64 =
        sqlx::query_scalar("select count(*) from modules where id = any($1)")
            .bind(&module_ids)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
    if module_ids.len() != module_values.len() || known != module_ids.len() as i64 {
        return validation_error(json!({"modules.0": ["The selected modules.0 is invalid."]}));
    }

    let Ok(Some(character_id)) = active_character(&pool, &session).await else {
        return back(&headers).into_response();
    };

    let collection = match collections::create_collection(
        &pool,
        character_id,
        payload.name.as_deref().unwrap_or_default(),
        payload.description.as_deref(),
        payload.visibility.as_deref().unwrap_or_default(),
    )
    .await
    {
        Ok(collection) => collection,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    for module_id in module_ids {
        if let Err(error) =
            collections::add_collection_module(&pool, collection.id, module_id, None).await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    }

    back(&headers).into_response()
}

/// Resolves the {collection} route segment (slug with trailing identifier)
/// and checks ownership; legacy authorize() answers 403.
async fn owned_collection(
    pool: &PgPool,
    headers: &HeaderMap,
    slug: &str,
) -> Result<collections::Collection, Response> {
    let session = require_session(pool, headers).await?;
    let collection = match collections::collection_by_slug(pool, slug).await {
        Ok(Some(collection)) => collection,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(error) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response());
        }
    };
    if !collection.owned_by(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(collection)
}

/// `PUT /collections/{collection}`
pub async fn update_collection(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let collection = match owned_collection(&pool, &headers, &slug).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };
    let payload: CollectionPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_collection(&payload) {
        return response;
    }

    match collections::update_collection(
        &pool,
        collection.id,
        payload.name.as_deref().unwrap_or_default(),
        payload.description.as_deref(),
        payload.visibility.as_deref().unwrap_or_default(),
    )
    .await
    {
        Ok(()) => {
            let slug = collections::Collection {
                name: payload.name.clone().unwrap_or_default(),
                ..collection
            }
            .slug();
            Redirect::to(&format!("/collections/{slug}")).into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// `DELETE /collections/{collection}`
pub async fn destroy_collection(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let collection = match owned_collection(&pool, &headers, &slug).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };

    match collections::delete_collection(&pool, collection.id).await {
        Ok(()) => Redirect::to("/collections").into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

#[derive(Deserialize, Default)]
struct CollectionModulePayload {
    collection_id: Option<i64>,
    module_id: Option<i64>,
    note: Option<String>,
}

async fn owned_collection_by_id(
    pool: &PgPool,
    headers: &HeaderMap,
    collection_id: i64,
) -> Result<collections::Collection, Response> {
    let session = require_session(pool, headers).await?;
    let collection = match collections::collection_by_id(pool, collection_id).await {
        Ok(Some(collection)) => collection,
        Ok(None) => {
            return Err(validation_error(
                json!({"collection_id": ["The selected collection id is invalid."]}),
            ));
        }
        Err(error) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response());
        }
    };
    if !collection.owned_by(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(collection)
}

/// `POST /collection-modules`
pub async fn store_collection_module(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let payload: CollectionModulePayload = serde_json::from_slice(&body).unwrap_or_default();
    let (Some(collection_id), Some(module_id)) = (payload.collection_id, payload.module_id) else {
        if let Err(response) = require_session(&pool, &headers).await {
            return response;
        }
        return validation_error(json!({
            "collection_id": ["The collection id field is required."],
            "module_id": ["The module id field is required."],
        }));
    };
    let collection = match owned_collection_by_id(&pool, &headers, collection_id).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };
    let module_exists: bool =
        sqlx::query_scalar("select exists (select 1 from modules where id = $1)")
            .bind(module_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    if !module_exists {
        return validation_error(json!({"module_id": ["The selected module id is invalid."]}));
    }

    match collections::add_collection_module(&pool, collection.id, module_id, payload.note.as_deref())
        .await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// `PUT /collection-modules/{collectionModule}` — updates the note.
pub async fn update_collection_module(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let collection_id: Option<i64> =
        sqlx::query_scalar("select collection_id from collection_modules where id = $1")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
    let Some(collection_id) = collection_id else {
        if let Err(response) = require_session(&pool, &headers).await {
            return response;
        }
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Err(response) = owned_collection_by_id(&pool, &headers, collection_id).await {
        return response;
    }

    let payload: CollectionModulePayload = serde_json::from_slice(&body).unwrap_or_default();
    match sqlx::query("update collection_modules set note = $1, updated_at = now() where id = $2")
        .bind(payload.note.as_deref())
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(_) => back(&headers).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// `DELETE /collection-modules/all`
pub async fn destroy_all_collection_modules(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let payload: CollectionModulePayload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(collection_id) = payload.collection_id else {
        if let Err(response) = require_session(&pool, &headers).await {
            return response;
        }
        return validation_error(json!({
            "collection_id": ["The collection id field is required."],
        }));
    };
    let collection = match owned_collection_by_id(&pool, &headers, collection_id).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };

    match sqlx::query("delete from collection_modules where collection_id = $1")
        .bind(collection.id)
        .execute(&pool)
        .await
    {
        Ok(_) => back(&headers).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// `DELETE /collection-modules/{collectionModule}`
pub async fn destroy_collection_module(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let collection_id: Option<i64> =
        sqlx::query_scalar("select collection_id from collection_modules where id = $1")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
    let Some(collection_id) = collection_id else {
        if let Err(response) = require_session(&pool, &headers).await {
            return response;
        }
        return StatusCode::NOT_FOUND.into_response();
    };
    if let Err(response) = owned_collection_by_id(&pool, &headers, collection_id).await {
        return response;
    }

    match sqlx::query("delete from collection_modules where id = $1").bind(id).execute(&pool).await {
        Ok(_) => back(&headers).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

#[derive(Deserialize, Default)]
struct CharacterPayload {
    description: Option<String>,
}

/// `PUT /characters/{character}` — the owner edits the bio description.
pub async fn update_character(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let session = match require_session(&pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let Some(character_id) = crate::characters::character_id_from_slug(&slug) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let owner: Option<Option<i64>> =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(character_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
    let Some(owner) = owner else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if owner != Some(session.user_id) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let payload: CharacterPayload = serde_json::from_slice(&body).unwrap_or_default();
    if payload.description.as_ref().is_some_and(|d| d.len() > CHARACTER_DESCRIPTION_MAX) {
        return validation_error(json!({
            "description": ["The description field must not be greater than 5000 characters."],
        }));
    }

    match sqlx::query("update characters set description = $1 where id = $2")
        .bind(payload.description.as_deref())
        .bind(character_id)
        .execute(&pool)
        .await
    {
        Ok(_) => back(&headers).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

/// The OpenGraph image endpoints. Unknown entities 404 like legacy;
/// deliberate divergence for known ones: legacy renders bespoke PNG cards,
/// we redirect to the EVE image server until the OG renderer is ported.
pub async fn og_module(State(pool): State<PgPool>, Path(id): Path<i64>) -> Response {
    let type_id: Option<i64> = sqlx::query_scalar("select type_id from modules where id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    match type_id {
        Some(type_id) => type_icon_redirect(type_id),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn og_type(State(pool): State<PgPool>, Path(id): Path<i64>) -> Response {
    let exists: bool = sqlx::query_scalar("select exists (select 1 from types where id = $1)")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

    if exists { type_icon_redirect(id) } else { StatusCode::NOT_FOUND.into_response() }
}

pub async fn og_character(State(pool): State<PgPool>, Path(id): Path<i64>) -> Response {
    let exists: bool = sqlx::query_scalar("select exists (select 1 from characters where id = $1)")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

    if exists {
        Redirect::temporary(&format!("https://images.evetech.net/characters/{id}/portrait"))
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn og_collection(State(pool): State<PgPool>, Path(id): Path<i64>) -> Response {
    let exists: bool = sqlx::query_scalar("select exists (select 1 from collections where id = $1)")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

    if exists {
        Redirect::temporary("/img/MutaMarket.png").into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

fn type_icon_redirect(type_id: i64) -> Response {
    Redirect::temporary(&format!("https://images.evetech.net/types/{type_id}/icon")).into_response()
}
