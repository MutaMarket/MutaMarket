//! Collection-location and auto-sync endpoints, ported from the legacy
//! `CollectionLocationController` (bulk add/sync/remove of a location's
//! modules) and `CollectionAutoSyncController` (enable/disable auto-sync
//! and manage its tracked locations). Success responses are the legacy
//! Inertia-style 302 back to the referer; failures mirror the legacy
//! order: 404 for an unknown collection or asset, 403 for a non-owner,
//! then the Laravel 422 shape for invalid payloads.
//!
//! Deliberate divergence: the legacy `ClearCollectionCacheAction` after
//! every auto-sync mutation has no counterpart because the rewrite reads
//! collections fresh per request (no shared-props cache).

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use super::social::{back, require_session, validation_error};
use crate::collections;

/// Whether an assets row with this primary key exists, the legacy
/// `exists:assets,id` rule. Legacy quirk kept: the rule checks the whole
/// assets table, not just the caller's own rows.
async fn asset_exists(pool: &PgPool, asset_id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar("select exists (select 1 from assets where id = $1)")
        .bind(asset_id)
        .fetch_one(pool)
        .await
}

fn database_error(error: sqlx::Error) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

#[derive(Deserialize, Default)]
struct CollectionLocationPayload {
    collection_id: Option<i64>,
    location_id: Option<i64>,
}

/// The shared front half of the /collection-locations endpoints, in the
/// legacy FormRequest order: session, findOrFail on collection_id (404),
/// the update policy (403), then the location_id rules (422).
async fn location_request(
    pool: &PgPool,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<(i64, collections::Collection, i64), Response> {
    let session = require_session(pool, headers).await?;
    let payload: CollectionLocationPayload = serde_json::from_slice(body).unwrap_or_default();

    // The legacy authorize() findOrFails the collection before any
    // validation runs, so a missing or unknown collection_id is a 404.
    let collection = match payload.collection_id {
        Some(collection_id) => match collections::collection_by_id(pool, collection_id).await {
            Ok(Some(collection)) => collection,
            Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
            Err(error) => return Err(database_error(error)),
        },
        None => return Err(StatusCode::NOT_FOUND.into_response()),
    };
    if !collection.owned_by(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }

    let Some(location_id) = payload.location_id else {
        return Err(validation_error(
            json!({"location_id": ["The location id field is required."]}),
        ));
    };
    match asset_exists(pool, location_id).await {
        Ok(true) => {}
        Ok(false) => {
            return Err(validation_error(
                json!({"location_id": ["The selected location id is invalid."]}),
            ));
        }
        Err(error) => return Err(database_error(error)),
    }

    Ok((session.user_id, collection, location_id))
}

/// `POST /collection-locations` — add every module inside the location.
pub async fn store(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::add_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(_) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `PUT /collection-locations` — replace the collection's modules with
/// the location's.
pub async fn put(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::sync_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `DELETE /collection-locations` — remove every module inside the
/// location from the collection.
pub async fn destroy(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::remove_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(_) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// The shared front half of the auto-sync endpoints: session, the
/// {collection} slug binding (404), then the update policy (403).
async fn owned_collection(
    pool: &PgPool,
    headers: &HeaderMap,
    slug: &str,
) -> Result<collections::Collection, Response> {
    let session = require_session(pool, headers).await?;
    let collection = match collections::collection_by_slug(pool, slug).await {
        Ok(Some(collection)) => collection,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(error) => return Err(database_error(error)),
    };
    if !collection.owned_by(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(collection)
}

#[derive(Deserialize, Default)]
struct EnableAutoSyncPayload {
    location_ids: Option<Vec<i64>>,
}

/// `POST /collections/{collection}/auto-sync` — enable auto-sync,
/// optionally seeding tracked locations.
pub async fn enable(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let collection = match owned_collection(&pool, &headers, &slug).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };

    let payload: EnableAutoSyncPayload = serde_json::from_slice(&body).unwrap_or_default();
    let location_ids = payload.location_ids.unwrap_or_default();
    // Each entry must be an existing assets.id (`exists:assets,id`); like
    // Laravel, every failing index is reported.
    let mut errors = serde_json::Map::new();
    for (index, asset_id) in location_ids.iter().enumerate() {
        match asset_exists(&pool, *asset_id).await {
            Ok(true) => {}
            Ok(false) => {
                errors.insert(
                    format!("location_ids.{index}"),
                    json!([format!("The selected location ids.{index} is invalid.")]),
                );
            }
            Err(error) => return database_error(error),
        }
    }
    if !errors.is_empty() {
        return validation_error(json!(errors));
    }

    match collections::enable_auto_sync(&pool, collection.id, collection.character_id, &location_ids)
        .await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `DELETE /collections/{collection}/auto-sync` — disable auto-sync;
/// the current modules are kept.
pub async fn disable(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let collection = match owned_collection(&pool, &headers, &slug).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };

    match collections::disable_auto_sync(&pool, collection.id).await {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

#[derive(Deserialize, Default)]
struct StoreAutoSyncLocationPayload {
    asset_id: Option<i64>,
}

/// `POST /collections/{collection}/auto-sync/locations` — track a
/// location and re-sync.
pub async fn store_location(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let collection = match owned_collection(&pool, &headers, &slug).await {
        Ok(collection) => collection,
        Err(response) => return response,
    };

    let payload: StoreAutoSyncLocationPayload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(asset_id) = payload.asset_id else {
        return validation_error(json!({"asset_id": ["The asset id field is required."]}));
    };
    match asset_exists(&pool, asset_id).await {
        Ok(true) => {}
        Ok(false) => {
            return validation_error(json!({"asset_id": ["The selected asset id is invalid."]}));
        }
        Err(error) => return database_error(error),
    }

    match collections::add_auto_sync_location(&pool, collection.id, collection.character_id, asset_id)
        .await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `DELETE /collections/{collection}/auto-sync/locations/{asset}` —
/// untrack a location and re-sync. The {asset} segment binds an assets.id
/// row like the legacy implicit binding: unknown ids (or non-numeric
/// segments, which Laravel would also findOrFail) answer 404, and both
/// bindings resolve before the update policy runs, so an unknown asset is
/// a 404 even for a non-owner.
pub async fn destroy_location(
    State(pool): State<PgPool>,
    Path((slug, asset)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let session = match require_session(&pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let collection = match collections::collection_by_slug(&pool, &slug).await {
        Ok(Some(collection)) => collection,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return database_error(error),
    };

    let Ok(asset_id) = asset.parse::<i64>() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match asset_exists(&pool, asset_id).await {
        Ok(true) => {}
        Ok(false) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return database_error(error),
    }

    if !collection.owned_by(session.user_id) {
        return StatusCode::FORBIDDEN.into_response();
    }

    match collections::remove_auto_sync_location(
        &pool,
        collection.id,
        collection.character_id,
        asset_id,
    )
    .await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}
