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

use super::AppState;
use crate::auth::session::{Session, session_from_headers};
use crate::collections::{self, COLLECTION_VISIBILITIES};
use crate::view::social::{
    CharacterCardData, CharacterPageData, CollectionCardData, CollectionPageData,
};

/// Longest collection name/description, the legacy max:255.
const COLLECTION_TEXT_MAX: usize = 255;

/// Longest character description, the legacy max:5000.
const CHARACTER_DESCRIPTION_MAX: usize = 5000;

pub(super) fn back(headers: &HeaderMap) -> Redirect {
    Redirect::to(
        headers.get(header::REFERER).and_then(|value| value.to_str().ok()).unwrap_or("/"),
    )
}

pub(super) fn validation_error(errors: serde_json::Value) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "message": "The given data was invalid.", "errors": errors })),
    )
        .into_response()
}

pub(super) async fn require_session(pool: &PgPool, headers: &HeaderMap) -> Result<Session, Response> {
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

/// Modules shown on a character or collection page, like the legacy
/// simplePaginate(40) page size.
const SOCIAL_MODULES_PAGE_SIZE: i64 = 40;

fn character_card(view: crate::characters::CharacterView) -> CharacterCardData {
    CharacterCardData {
        id: view.id,
        slug: view.slug,
        name: view.name,
        description: view.description,
        has_premium: view.has_premium,
        corporation_id: view.corporation_id,
        modules_count: view.modules_count,
    }
}

/// The character index cards shared with the Leptos server function.
pub async fn character_cards(
    state: &AppState,
    search: Option<&str>,
) -> sqlx::Result<Vec<CharacterCardData>> {
    crate::characters::characters_index(&state.pool, search, 1)
        .await
        .map(|characters| characters.into_iter().map(character_card).collect())
}

/// The character page payload shared with the Leptos server function;
/// `None` marks an unknown slug or id.
pub async fn character_page_data(
    state: &AppState,
    slug: &str,
    query: &str,
) -> Result<Option<CharacterPageData>, crate::modules::search::SearchError> {
    use crate::modules::search::{Scope, parse, scoped_module_ids};

    let Some(id) = crate::characters::character_id_from_slug(slug) else {
        return Ok(None);
    };
    let Some(character) = crate::characters::character_by_id(&state.pool, id)
        .await
        .map_err(crate::modules::search::SearchError::Db)?
    else {
        return Ok(None);
    };

    // The full filter grammar applies, scoped to the character: public
    // listings by default, creations with the `created` option (the
    // legacy CharacterController show).
    let search = parse(&state.pool, &state.reference, query).await?;
    let scope = if search.created { Scope::CreatedBy(id) } else { Scope::Character(id) };
    let ids = scoped_module_ids(&state.pool, &search, scope, SOCIAL_MODULES_PAGE_SIZE)
        .await
        .map_err(crate::modules::search::SearchError::Db)?;
    let modules = crate::modules::queries::details_for(&state.pool, &state.reference, ids)
        .await
        .map_err(crate::modules::search::SearchError::Db)?;

    // Header stats over the character's whole sets (the same conditions
    // as the Character/CreatedBy scopes), unaffected by page filters.
    let (for_sale_count, created_count): (i64, i64) = sqlx::query_as(
        "select
             (select count(*) from modules m where exists (
                  select 1 from public_module_ownerships o
                  where o.module_id = m.id and o.character_id = $1
              )),
             (select count(*) from modules m where m.creator_id = $1)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(crate::modules::search::SearchError::Db)?;

    Ok(Some(CharacterPageData {
        character: character_card(character),
        modules,
        for_sale_count,
        created_count,
    }))
}

/// Type icons shown per collection card (the legacy card slices its
/// types client-side; the cap keeps the payload bounded).
const CARD_TYPE_ICONS: usize = 8;

async fn listing_cards(
    pool: &sqlx::PgPool,
    listings: Vec<collections::CollectionListing>,
) -> sqlx::Result<Vec<CollectionCardData>> {
    let ids: Vec<i64> = listings.iter().map(|listing| listing.collection.id).collect();
    let mut types = collections::collection_type_ids(pool, &ids).await?;

    Ok(listings
        .into_iter()
        .map(|listing| {
            let all_types = types.remove(&listing.collection.id).unwrap_or_default();
            let types_count = all_types.len() as i64;
            CollectionCardData {
                id: listing.collection.id,
                slug: listing.collection.slug(),
                name: listing.collection.name.clone(),
                description: listing.collection.description.clone(),
                visibility: listing.collection.visibility.clone(),
                character_id: listing.collection.character_id,
                character_name: listing.character_name,
                character_has_premium: listing.character_has_premium,
                modules_count: listing.modules_count,
                type_ids: all_types.into_iter().take(CARD_TYPE_ICONS).collect(),
                types_count,
            }
        })
        .collect())
}

/// The collection index cards shared with the Leptos server function.
pub async fn collection_cards(
    state: &AppState,
    search: Option<&str>,
) -> sqlx::Result<Vec<CollectionCardData>> {
    let listings = collections::collections_index(&state.pool, search, 1).await?;
    listing_cards(&state.pool, listings).await
}

/// The logged-in user's own collections for the index's personal
/// section.
pub async fn personal_collection_cards(
    state: &AppState,
    user_id: i64,
) -> sqlx::Result<Vec<CollectionCardData>> {
    let listings = collections::collections_index_for_user(&state.pool, user_id).await?;
    listing_cards(&state.pool, listings).await
}

/// The collection page outcomes, carrying the legacy 403 for a known but
/// private collection viewed by a non-owner.
pub enum CollectionPageOutcome {
    Page(Box<CollectionPageData>),
    Forbidden,
    NotFound,
}

/// The collection page payload shared with the Leptos server function.
pub async fn collection_page_data(
    state: &AppState,
    slug: &str,
    user_id: Option<i64>,
    query: &str,
) -> sqlx::Result<CollectionPageOutcome> {
    let Some(collection) = collections::collection_by_slug(&state.pool, slug).await? else {
        return Ok(CollectionPageOutcome::NotFound);
    };
    if !collection.viewable_by(user_id) {
        return Ok(CollectionPageOutcome::Forbidden);
    }

    let search = crate::modules::search::parse(&state.pool, &state.reference, query)
        .await
        .map_err(|error| match error {
            crate::modules::search::SearchError::Db(db_error) => db_error,
            // Grammar failures on a collection page degrade to the
            // unfiltered set rather than erroring the whole page.
            _ => sqlx::Error::RowNotFound,
        });
    let ids = match search {
        Ok(search) => crate::modules::search::scoped_module_ids(
            &state.pool,
            &search,
            crate::modules::search::Scope::Collection(collection.id),
            SOCIAL_MODULES_PAGE_SIZE,
        )
        .await?,
        Err(_) => {
            let mut ids = collections::collection_module_ids(&state.pool, collection.id).await?;
            ids.truncate(SOCIAL_MODULES_PAGE_SIZE as usize);
            ids
        }
    };
    let modules = crate::modules::queries::details_for(&state.pool, &state.reference, ids).await?;

    let character_name: String = sqlx::query_scalar("select name from characters where id = $1")
        .bind(collection.character_id)
        .fetch_one(&state.pool)
        .await?;

    // Header stats over the whole collection (the page above is
    // filter-scoped and capped).
    let (modules_count, estimated_value_total): (i64, f64) = sqlx::query_as(
        "select count(*), coalesce(sum(m.estimated_value), 0)
         from collection_modules cm
         join modules m on m.id = cm.module_id
         where cm.collection_id = $1",
    )
    .bind(collection.id)
    .fetch_one(&state.pool)
    .await?;

    let character_has_premium: bool = sqlx::query_scalar(
        "select premium_paid_until is not null and premium_paid_until > now()
         from characters where id = $1",
    )
    .bind(collection.character_id)
    .fetch_one(&state.pool)
    .await?;
    let mut types =
        collections::collection_type_ids(&state.pool, &[collection.id]).await?;
    let all_types = types.remove(&collection.id).unwrap_or_default();
    let types_count = all_types.len() as i64;

    let (auto_sync, last_synced_at): (bool, Option<String>) = sqlx::query_as(
        "select auto_sync, last_synced_at::text from collections where id = $1",
    )
    .bind(collection.id)
    .fetch_one(&state.pool)
    .await?;

    // The manage-modules data is owner-only, like the legacy
    // getLocationsIfAuthorized and the owner-gated collectionLocations
    // loadout.
    let is_owner = user_id.is_some() && user_id == collection.owner_user_id;
    let (tracked_locations, locations) = if is_owner {
        let tracked_ids: Vec<i64> = sqlx::query_scalar(
            "select asset_id from collection_locations where collection_id = $1 order by id",
        )
        .bind(collection.id)
        .fetch_all(&state.pool)
        .await?;
        (
            Some(
                crate::assets::location_views_for_assets(
                    &state.pool,
                    collection.character_id,
                    &tracked_ids,
                )
                .await?,
            ),
            Some(crate::assets::character_locations(&state.pool, collection.character_id).await?),
        )
    } else {
        (None, None)
    };

    Ok(CollectionPageOutcome::Page(Box::new(CollectionPageData {
        collection: CollectionCardData {
            id: collection.id,
            slug: collection.slug(),
            name: collection.name.clone(),
            description: collection.description.clone(),
            visibility: collection.visibility.clone(),
            character_id: collection.character_id,
            character_name,
            character_has_premium,
            modules_count,
            type_ids: all_types.into_iter().take(CARD_TYPE_ICONS).collect(),
            types_count,
        },
        modules,
        estimated_value_total,
        auto_sync,
        last_synced_at,
        tracked_locations,
        locations,
    })))
}

#[derive(Deserialize, Default)]
pub struct SocialSearchParams {
    pub personal: Option<bool>,
    search: Option<String>,
}

/// `GET /api/characters?search=` — the character index cards.
pub async fn characters_index(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<SocialSearchParams>,
) -> Response {
    match character_cards(&state, params.search.as_deref()).await {
        Ok(cards) => Json(cards).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `GET /api/characters/{character}` — the character page payload.
pub async fn character_show(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(params): axum::extract::Query<PageQueryParams>,
) -> Response {
    use crate::modules::search::SearchError;

    match character_page_data(&state, &slug, params.q.as_deref().unwrap_or("")).await {
        Ok(Some(page)) => Json(page).into_response(),
        Ok(None) => super::api::error(StatusCode::NOT_FOUND, "Character not found"),
        Err(SearchError::Db(error)) => super::api::database_error(error),
        Err(SearchError::TypeNotFound) => {
            super::api::error(StatusCode::NOT_FOUND, "Please provide a valid type.")
        }
        Err(error) => super::api::error(StatusCode::BAD_REQUEST, &error.to_string()),
    }
}

/// The optional filter grammar carried by the scoped module pages.
#[derive(serde::Deserialize, Default)]
pub struct PageQueryParams {
    pub q: Option<String>,
}

/// `GET /api/collections?search=` — the collection index cards.
pub async fn collections_index(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SocialSearchParams>,
) -> Response {
    // ?personal=true: the caller's own collections, every visibility
    // (the legacy personal_collections index section).
    if params.personal.unwrap_or(false) {
        let session =
            match crate::auth::session::session_from_headers(&state.pool, &headers).await {
                Ok(Some(session)) => session,
                Ok(None) => {
                    return super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated.");
                }
                Err(error) => return super::api::database_error(error),
            };
        return match personal_collection_cards(&state, session.user_id).await {
            Ok(cards) => Json(cards).into_response(),
            Err(error) => super::api::database_error(error),
        };
    }

    match collection_cards(&state, params.search.as_deref()).await {
        Ok(cards) => Json(cards).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `GET /api/collections/module/{module}` — the signed-in user's
/// collections with the module's membership, powering the module menu's
/// collections submenu (the legacy `useCollections` composable read the
/// same pairing from shared Inertia props).
pub async fn collections_for_module(
    State(state): State<AppState>,
    axum::extract::Path(module_id): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated."),
        Err(error) => return super::api::database_error(error),
    };

    type Row = (i64, String, String, Option<i64>);
    let rows: Result<Vec<Row>, _> = sqlx::query_as(
        "select c.id, c.name, c.identifier, cm.id as collection_module_id
         from collections c
         join characters ch on ch.id = c.character_id
         left join collection_modules cm
           on cm.collection_id = c.id and cm.module_id = $2
         where ch.user_id = $1
         order by c.name, c.id",
    )
    .bind(session.user_id)
    .bind(module_id)
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => axum::Json(
            rows.into_iter()
                .map(|(id, name, identifier, collection_module_id)| {
                    json!({
                        "id": id,
                        "name": name,
                        "slug": format!(
                            "{}-{identifier}",
                            crate::modules::view::slugify(&name),
                        ),
                        "collection_module_id": collection_module_id,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `GET /api/collections/{collection}` — the collection page payload,
/// with the legacy 403 for a private collection viewed by a non-owner.
pub async fn collection_show(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<PageQueryParams>,
) -> Response {
    let user_id = match session_from_headers(&state.pool, &headers).await {
        Ok(session) => session.map(|session| session.user_id),
        Err(error) => return super::api::database_error(error),
    };

    match collection_page_data(&state, &slug, user_id, params.q.as_deref().unwrap_or("")).await {
        Ok(CollectionPageOutcome::Page(page)) => Json(*page).into_response(),
        Ok(CollectionPageOutcome::Forbidden) => {
            super::api::error(StatusCode::FORBIDDEN, "This collection is private.")
        }
        Ok(CollectionPageOutcome::NotFound) => {
            super::api::error(StatusCode::NOT_FOUND, "Collection not found")
        }
        Err(error) => super::api::database_error(error),
    }
}
